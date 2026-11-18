#![allow(dead_code)]

use macroquad::prelude::*;

mod ai;
mod audio;
mod content;
mod core;
mod ecs;
mod network;
mod protocol;
mod render;

use crate::audio::AudioManager;

use crate::content::mobs::{get_squad_config, spawn_squad_perimeter};
use crate::content::structures::{
    can_place_blood_altar, can_place_wall, spawn_blood_altar, spawn_corner_obelisks,
    snap_to_build_grid, try_spawn_tier_altar, try_spawn_wall,
    altar_cost, altar_cooldown, altar_tier_from_structure_type, structure_type_for_altar_tier,
    WALL_COST,
    STRUCTURE_WALL,
};
use crate::core::time::GameClock;

#[derive(PartialEq, Clone, Copy, Debug)]
enum PlacementMode {
    None,
    Altar,
    Wall,
}
use crate::core::{MAP_HEIGHT, MAP_WIDTH};
use crate::ecs::systems::{
    boss_ai::update_boss_ai,
    director::update_director,
    movement::update_movement,
};
use crate::ecs::world::World;
use crate::network::driver::NetworkDriver;
use crate::network::session::UdpDriver;
use crate::protocol::commands::Command;
use crate::render::codex::draw_codex;
use crate::render::cursor::draw_overlord_cursor;
use crate::render::dossier::draw_mob_dossier;
use crate::render::font::GameFonts;
use crate::render::textures::GameTextures;
use crate::render::{renderer::render_world, ui::render_ui};

fn window_conf() -> Conf {
    Conf {
        window_title: "Reverse Survivor: The Director".to_string(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Режим автономного со-эволюционного обучения ботов (Self-Play Training)
    // Supports --train [generations] [population] [duration] [--bosses N] [--think-ms 50|100|200] [--clean / --reset].
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--train") {
        let generations = args.get(pos + 1).and_then(|s| s.parse().ok()).unwrap_or(8);
        let population = args.get(pos + 2).and_then(|s| s.parse().ok()).unwrap_or(8);
        let boss_count = args.iter()
            .position(|arg| arg == "--bosses" || arg == "--boss-count")
            .and_then(|idx| args.get(idx + 1))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        let reset = args.iter().any(|a| a == "--clean" || a == "--reset");
        let training_think_interval = args
            .iter()
            .position(|a| a == "--think-ms")
            .and_then(|idx| args.get(idx + 1))
            .and_then(|value| value.parse::<f32>().ok())
            .map(|ms| (ms / 1000.0).clamp(0.05, 0.20))
            .unwrap_or(crate::ai::neural_net::BOSS_TRAINING_DECISION_INTERVAL);

        // Выбор длительности раунда матчей (300 сек, 1020 сек или произвольный)
        let round_duration = if let Some(cap_pos) = args.iter().position(|a| a == "--cap" || a == "--duration") {
            args.get(cap_pos + 1).and_then(|s| s.parse::<f32>().ok())
        } else if args.iter().any(|a| a == "--300" || a == "--fast") {
            Some(300.0)
        } else if args.iter().any(|a| a == "--1020" || a == "--full") {
            Some(1020.0)
        } else {
            // Позиционный 3-й аргумент: cargo run --release -- --train 20 16 300
            args.get(pos + 3).and_then(|s| s.parse::<f32>().ok())
        };

        crate::ai::trainer::run_co_evolutionary_training(
            generations,
            population,
            reset,
            round_duration,
            training_think_interval,
            boss_count,
        );
        return;
    }

    let boss_model = crate::ai::load_trained_boss_model();
    let director_model = crate::ai::load_trained_director_model()
        .unwrap_or_else(crate::ai::models::DirectorModel::default);
    let mut menu = crate::render::main_menu::MainMenu::load();
    let args: Vec<String> = std::env::args().collect();
    let has_host = args.iter().any(|arg| arg == "--host");
    let connect_arg = args.iter().position(|arg| arg == "--connect").and_then(|index| args.get(index + 1));
    let mut game_role = if !has_host && (args.iter().any(|a| a == "--boss-player") || connect_arg.is_some()) {
        crate::render::main_menu::GameRole::Boss
    } else {
        crate::render::main_menu::GameRole::Director
    };
    let show_main_menu_at_start = !args.iter().any(|a| {
        a == "--screenshot" || a == "--codex" || a == "--dossier" || a == "--recipe"
            || a == "--dossier-item" || a == "--settings" || a == "--pause" || a == "--boss-player"
            || a == "--host" || a == "--connect"
    });
    let mut show_main_menu = show_main_menu_at_start;
    let mut world = World::new();
    // Модель загружается один раз за сессию, а не на каждом создании мира.
    world.boss_model = boss_model.clone();
    apply_player_profile(&mut world, &menu.settings, game_role);
    let cli_match_duration = args.iter()
        .position(|arg| arg == "--match-minutes")
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|minutes| (1.0..=180.0).contains(minutes))
        .map(|minutes| minutes * 60.0)
        .unwrap_or(crate::core::time::GAME_DURATION_SECS);
    let mut clock = GameClock::with_duration(if has_host { cli_match_duration } else { crate::core::time::GAME_DURATION_SECS });
    world.match_duration_secs = clock.duration_secs;
    let mut net_driver = if has_host {
        let bind = args.iter().position(|arg| arg == "--host")
            .and_then(|index| args.get(index + 1))
            .filter(|value| !value.starts_with("--"))
            .map(String::as_str)
            .unwrap_or("0.0.0.0:45678");
        match UdpDriver::host(bind) {
            Ok(driver) => driver,
            Err(error) => { eprintln!("Could not start UDP host on {bind}: {error}"); return; }
        }
    } else if let Some(address) = connect_arg {
        let Ok(address) = address.parse() else {
            eprintln!("Invalid --connect address; expected an IP address such as 26.12.34.56:45678");
            return;
        };
        match UdpDriver::client(address, menu.settings.nickname.clone(), menu.settings.skin) {
            Ok(driver) => driver,
            Err(error) => { eprintln!("Could not start UDP client: {error}"); return; }
        }
    } else {
        UdpDriver::local()
    };
    let fonts = GameFonts::load();
    let textures = GameTextures::load().await;
    let mut audio = AudioManager::load().await;
    let auto_screenshot = std::env::args().any(|a| a == "--screenshot");
    let auto_codex = std::env::args().any(|a| a == "--codex");
    let auto_dossier = std::env::args().any(|a| a == "--dossier");
    let auto_recipe = std::env::args().any(|a| a == "--recipe" || a == "--dossier-item");
    let auto_settings = std::env::args().any(|a| a == "--settings" || a == "--pause");
    let mut is_paused = auto_settings;
    let mut director_ai_timer = 0.0f32;
    let mut boss_death_sound_played = false;

    // Скрываем стандартный системный курсор мыши (заменяем демонической дланью Повелителя)
    show_mouse(show_main_menu || game_role == crate::render::main_menu::GameRole::Boss);

    // Камера свободной навигации по большой карте
    let mut cam_pos = world.boss_pos;
    let mut cam_zoom: f32 = 0.95;
    let mut follow_boss = true;
    let mut show_codex = auto_codex;
    let mut codex_tab: usize = std::env::args()
        .position(|a| a == "--codex-tab")
        .and_then(|idx| std::env::args().nth(idx + 1))
        .and_then(|val| val.parse().ok())
        .unwrap_or(0);
    let mut frame_count = 0;

    let mut last_mouse_pos = Vec2::from(mouse_position());
    let mut is_dragging_map = false;
    let mut placement_mode = PlacementMode::None;
    let mut selected_altar_tier = 1u8;
    let mut active_dossier_tier: Option<u8> = if auto_dossier { Some(1) } else { None };
    let mut opened_dossier_from_codex = false;
    let mut active_item_inspection: Option<crate::render::item_dossier::ItemInspectionTarget> = if auto_recipe {
        Some(crate::render::item_dossier::ItemInspectionTarget::Weapon(crate::ecs::components::inventory::WeaponType::Hellfire, 8, 0))
    } else {
        None
    };
    let mut opened_item_from_codex = false;

    // Начальный разрушаемый алтарь игрока и 4 древних угловых обелиска
    spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
    spawn_corner_obelisks(&mut world);

    // Запуск динамичного джазового / симфонического плейлиста (со случайного трека)
    if !auto_screenshot && !auto_codex && !auto_dossier && !auto_recipe && !auto_settings {
        audio.shuffle_start();
    }

    loop {
        let dt = get_frame_time().min(0.05);
        frame_count += 1;
        let current_mpos = Vec2::from(mouse_position());
        let mouse_delta = current_mpos - last_mouse_pos;

        if !show_main_menu
            && game_role == crate::render::main_menu::GameRole::Director
            && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
        {
            let altar_tier_key = [
                (KeyCode::Q, 1), (KeyCode::W, 2), (KeyCode::E, 3), (KeyCode::R, 4),
                (KeyCode::A, 5), (KeyCode::S, 6), (KeyCode::D, 7), (KeyCode::F, 8),
                (KeyCode::Z, 9), (KeyCode::X, 10), (KeyCode::C, 11), (KeyCode::V, 12),
                (KeyCode::G, 13), (KeyCode::H, 14), (KeyCode::Y, 15), (KeyCode::U, 16),
            ];
            if let Some((_, tier)) = altar_tier_key.iter().find(|(key, _)| is_key_pressed(*key)) {
                selected_altar_tier = *tier;
                placement_mode = PlacementMode::Altar;
                world.add_floating_text(
                    world.boss_pos,
                    format!("Выбран алтарь тира {}", selected_altar_tier),
                    GOLD,
                    1.1,
                );
            }
        }

        if show_main_menu {
            if let Some(action) = menu.draw(&fonts) {
                let launch: Result<(_, _, f32), String> = match action {
                    crate::render::main_menu::MenuAction::Local(role) => {
                        Ok((role, UdpDriver::local(), crate::core::time::GAME_DURATION_SECS))
                    }
                    crate::render::main_menu::MenuAction::Host { bind_address, duration_secs } => {
                        UdpDriver::host(&bind_address)
                            .map(|driver| (crate::render::main_menu::GameRole::Director, driver, duration_secs))
                            .map_err(|error| format!("Не удалось создать игру на {bind_address}: {error}"))
                    }
                    crate::render::main_menu::MenuAction::Join { address } => {
                        match address.parse() {
                            Ok(host) => UdpDriver::client(host, menu.settings.nickname.clone(), menu.settings.skin)
                                .map(|driver| (crate::render::main_menu::GameRole::Boss, driver, crate::core::time::GAME_DURATION_SECS))
                                .map_err(|error| format!("Не удалось подключиться к {address}: {error}")),
                            Err(_) => Err("Нужен адрес хоста в формате IP:порт, например 26.12.34.56:45678.".to_string()),
                        }
                    }
                };

                match launch {
                    Ok((role, driver, duration_secs)) => {
                        net_driver = driver;
                        game_role = role;
                        show_main_menu = false;
                        world = World::new();
                        world.boss_model = boss_model.clone();
                        world.match_duration_secs = duration_secs;
                        apply_player_profile(&mut world, &menu.settings, game_role);
                        clock = GameClock::with_duration(duration_secs);
                        cam_pos = world.boss_pos;
                        follow_boss = true;
                        spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
                        spawn_corner_obelisks(&mut world);
                        audio.shuffle_start();
                        show_mouse(game_role == crate::render::main_menu::GameRole::Boss);
                    }
                    Err(error) => menu.set_notice(error),
                }
            }
            if std::env::args().any(|a| a == "--menu-screenshot") && frame_count >= 10 {
                get_screen_data().export_png("screenshot_menu.png");
                break;
            }
            last_mouse_pos = current_mpos;
            next_frame().await;
            continue;
        }

        if auto_screenshot && frame_count == 10 {
            world.boss_inventory.weapons.push(crate::ecs::components::inventory::WeaponSlot {
                kind: crate::ecs::components::inventory::WeaponType::Hellfire,
                level: 8,
                cooldown_timer: 0.0,
                base_cooldown: 0.5,
                damage: 48.0,
                is_evolved: true,
                limit_break_level: 0,
                bonus_projectiles: 0,
            });
            world.boss_inventory.weapons.push(crate::ecs::components::inventory::WeaponSlot {
                kind: crate::ecs::components::inventory::WeaponType::HolyHammer,
                level: 3,
                cooldown_timer: 0.0,
                base_cooldown: 1.2,
                damage: 35.0,
                is_evolved: false,
                limit_break_level: 0,
                bonus_projectiles: 0,
            });
            world.boss_inventory.weapons.push(crate::ecs::components::inventory::WeaponSlot {
                kind: crate::ecs::components::inventory::WeaponType::ThunderOrb,
                level: 2,
                cooldown_timer: 0.0,
                base_cooldown: 1.0,
                damage: 25.0,
                is_evolved: false,
                limit_break_level: 0,
                bonus_projectiles: 0,
            });
            world.boss_inventory.passives.push(crate::ecs::components::inventory::PassiveSlot {
                kind: crate::ecs::components::inventory::PassiveType::GiantHeart,
                level: 3,
            });
            world.boss_inventory.passives.push(crate::ecs::components::inventory::PassiveSlot {
                kind: crate::ecs::components::inventory::PassiveType::LightArmor,
                level: 2,
            });
            world.boss_inventory.passives.push(crate::ecs::components::inventory::PassiveSlot {
                kind: crate::ecs::components::inventory::PassiveType::BloodStone,
                level: 1,
            });
            world.boss_inventory.passives.push(crate::ecs::components::inventory::PassiveSlot {
                kind: crate::ecs::components::inventory::PassiveType::EagleScope,
                level: 4,
            });

            spawn_squad_perimeter(&mut world, 1);
            spawn_squad_perimeter(&mut world, 3);
            spawn_squad_perimeter(&mut world, 6);
            spawn_squad_perimeter(&mut world, 7);
            spawn_squad_perimeter(&mut world, 8);

            // Спавним отряды в радиусе слышимости джаза (240-320px) для демонстрации подтанцовки
            crate::content::mobs::spawn_squad_at_dist(&mut world, 1, 230.0);
            crate::content::mobs::spawn_squad_at_dist(&mut world, 2, 310.0);
        }

        // Переключение звука по M
        if is_key_pressed(KeyCode::M) {
            audio.toggle_mute();
        }

        // Инфернальный кодекс по I
        if is_key_pressed(KeyCode::I) {
            if active_item_inspection.is_some() {
                active_item_inspection = None;
                opened_item_from_codex = false;
                show_codex = false;
                audio.handle_event(crate::audio::AudioEvent::BookClose);
            } else if active_dossier_tier.is_some() {
                active_dossier_tier = None;
                opened_dossier_from_codex = false;
                show_codex = false;
                audio.handle_event(crate::audio::AudioEvent::BookClose);
            } else {
                show_codex = !show_codex;
                if show_codex {
                    active_dossier_tier = None;
                    active_item_inspection = None;
                    audio.handle_event(crate::audio::AudioEvent::BookOpen);
                } else {
                    audio.handle_event(crate::audio::AudioEvent::BookClose);
                }
            }
        }
        // Закрытие окон или открытие настроек по Esc
        if is_key_pressed(KeyCode::Escape) {
            if active_item_inspection.is_some() {
                active_item_inspection = None;
                audio.handle_event(crate::audio::AudioEvent::BookClose);
                if opened_item_from_codex {
                    show_codex = true;
                    opened_item_from_codex = false;
                }
            } else if active_dossier_tier.is_some() {
                active_dossier_tier = None;
                audio.handle_event(crate::audio::AudioEvent::BookClose);
                if opened_dossier_from_codex {
                    show_codex = true;
                    opened_dossier_from_codex = false;
                }
            } else if show_codex {
                show_codex = false;
                audio.handle_event(crate::audio::AudioEvent::BookClose);
            } else {
                is_paused = !is_paused;
                audio.handle_event(crate::audio::AudioEvent::UiClick);
            }
        }

        // Скриншот по F12
        if is_key_pressed(KeyCode::F12) {
            get_screen_data().export_png("screenshot.png");
        }

        // Перезапуск игры по R
        let boss_team_alive = world.any_boss_alive();
        let is_game_over = !boss_team_alive || clock.is_time_up();
        if !boss_team_alive && !boss_death_sound_played {
            audio.handle_event(crate::audio::AudioEvent::BossDeath);
            boss_death_sound_played = true;
        }

        // Управление плейлистом: N — след. трек, P — пред. трек
        if is_key_pressed(KeyCode::N) {
            audio.next_track();
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }
        if is_key_pressed(KeyCode::P) {
            audio.prev_track();
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }

        // 1. Управление камерой на большой карте: WASD + колесо мыши + центрирование по C
        handle_camera_input(
            &mut cam_pos,
            &mut cam_zoom,
            &mut follow_boss,
            world.boss_pos,
            dt,
            game_role == crate::render::main_menu::GameRole::Boss,
        );

        let screen_w = screen_width();
        let screen_h = screen_height();
        let world_cam = Camera2D {
            target: cam_pos,
            zoom: Vec2::new(cam_zoom / (screen_w / 2.0), cam_zoom / (screen_h / 2.0)),
            ..Default::default()
        };

        let is_running = !is_game_over && !is_paused && !show_codex && active_dossier_tier.is_none();

        // 2. Сбор ввода игрока-директора -> генерация Команд
        let mut player_move_dir = Vec2::ZERO;
        let mut player_aim_dir = Vec2::ZERO;
        let mut player_dash_pressed = false;
        if is_running {
            clock.tick(dt);
            match game_role {
                crate::render::main_menu::GameRole::Director => {
                    handle_player_input(&world, &mut net_driver, &world_cam);
                }
                crate::render::main_menu::GameRole::Boss => {
                    (player_move_dir, player_aim_dir, player_dash_pressed) =
                        collect_boss_player_input(&menu.settings, &world, &world_cam);
                    if net_driver.is_network_client() {
                        if let Some(player_id) = net_driver.assigned_player_id() {
                            net_driver.send_command(Command::BossInput {
                                player_id,
                                move_dir: player_move_dir.into(),
                                aim_dir: player_aim_dir.into(),
                                dash_pressed: player_dash_pressed,
                            });
                        }
                    }
                }
            }
        }
        if !is_running && net_driver.is_network_client() && game_role == crate::render::main_menu::GameRole::Boss {
            if let Some(player_id) = net_driver.assigned_player_id() {
                net_driver.send_command(Command::BossInput {
                    player_id,
                    move_dir: Vec2::ZERO.into(),
                    aim_dir: world.boss_facing.into(),
                    dash_pressed: false,
                });
            }
        }

        // 3. Исполнение сетевых/локальных команд (Command Pattern)
        let commands = net_driver.poll_commands();
        for cmd in commands {
            execute_command(&mut world, cmd);
        }
        if net_driver.is_network_client() {
            if let Some(snapshot) = net_driver.take_snapshot() {
                let local_id = net_driver.assigned_player_id().unwrap_or(1);
                crate::network::snapshot::apply_to_world(snapshot, &mut world, local_id);
                clock.elapsed_secs = world.game_time;
                clock.duration_secs = world.match_duration_secs;
                if follow_boss { cam_pos = world.boss_pos; }
            }
        }

        // 4. Обновление систем мира
        if is_running && !net_driver.is_network_client() {
            update_director(&mut world, dt);
            match game_role {
                crate::render::main_menu::GameRole::Director => {
                    if world.boss_human_controlled {
                        let move_dir = world.boss_input_move_dir;
                        let aim_dir = world.boss_input_aim_dir;
                        let dash_pressed = world.boss_input_dash_pressed;
                        crate::ecs::systems::boss_ai::update_player_boss(
                            &mut world,
                            dt,
                            move_dir,
                            aim_dir,
                            dash_pressed,
                        );
                        world.boss_input_dash_pressed = false;
                    } else {
                        update_boss_ai(&mut world, dt);
                    }
                }
                crate::render::main_menu::GameRole::Boss => {
                    director_ai_timer += dt;
                    if director_ai_timer >= 0.4 {
                        director_ai_timer -= 0.4;
                        crate::ai::simulation::update_director_policy(
                            &mut world,
                            clock.elapsed_secs,
                            &director_model,
                        );
                    }
                    crate::ecs::systems::boss_ai::update_player_boss(
                        &mut world,
                        dt,
                        player_move_dir,
                        player_aim_dir,
                        player_dash_pressed,
                    );
                }
            }
            for index in 0..world.coop_bosses.len() {
                let (human_controlled, move_dir, aim_dir, dash_pressed) = {
                    let player = &world.coop_bosses[index];
                    (player.human_controlled, player.input_move_dir, player.input_aim_dir, player.input_dash_pressed)
                };
                if world.swap_coop_boss_context(index) {
                    if human_controlled {
                    crate::ecs::systems::boss_ai::update_player_boss(
                        &mut world,
                        dt,
                        move_dir,
                        aim_dir,
                        dash_pressed,
                    );
                    } else {
                        update_boss_ai(&mut world, dt);
                    }
                    if let Some(player) = world.coop_bosses.get_mut(index) {
                        player.input_dash_pressed = false;
                    }
                    world.swap_coop_boss_context(index);
                }
            }
            crate::ecs::systems::coop::tick_coop_boss_timers(&mut world, dt);
            crate::ecs::systems::coop::update_all_boss_effects(&mut world, dt);
            update_movement(&mut world, dt);
            crate::ecs::systems::coop::update_coop_boss_environment(&mut world, dt);
            crate::ecs::systems::coop::update_all_boss_combat(&mut world, dt);
            crate::ecs::systems::coop::update_revives(&mut world, dt);
            audio.process_events(&mut world);
            audio.update_music(dt, cam_pos, world.boss_pos);
        } else {
            audio.update_music(dt, cam_pos, world.boss_pos);
        }
        net_driver.publish_snapshot(&world);

        // 5. Отрисовка мира через камеру с Viewport Frustum Culling
        set_camera(&world_cam);
        render_world(&world, &fonts, &textures, cam_pos, cam_zoom);

        // Призрак Структуры при интерактивном режиме установки
        if placement_mode != PlacementMode::None {
            draw_build_grid_overlay(cam_pos, cam_zoom, screen_w, screen_h);
            let (mx, my) = mouse_position();
            let world_cursor = world_cam.screen_to_world(Vec2::new(mx, my));
            let clamped = snap_to_build_grid(world_cursor);

            match placement_mode {
                PlacementMode::Altar => {
                    let can_place = can_place_blood_altar(&world, clamped);
                    let border_color = if can_place { Color::new(0.2, 1.0, 0.4, 0.9) } else { Color::new(1.0, 0.2, 0.2, 0.9) };
                    let fill_color = if can_place { Color::new(0.2, 0.9, 0.4, 0.25) } else { Color::new(0.9, 0.2, 0.2, 0.25) };

                    draw_circle_lines(clamped.x, clamped.y, 120.0, 1.5, Color::new(border_color.r, border_color.g, border_color.b, 0.35));
                    draw_circle(clamped.x, clamped.y, 30.0, fill_color);
                    draw_circle_lines(clamped.x, clamped.y, 30.0, 2.5, border_color);

                    if let Some(tex) = textures.altar.as_ref() {
                        let w = 48.0;
                        let h = 70.0;
                        let tint = if can_place { Color::new(0.8, 1.0, 0.8, 0.75) } else { Color::new(1.0, 0.5, 0.5, 0.75) };
                        draw_texture_ex(tex, clamped.x - w * 0.5, clamped.y - h * 0.5, tint, DrawTextureParams {
                            dest_size: Some(Vec2::new(w, h)),
                            ..Default::default()
                        });
                    }
                    let cost = altar_cost(selected_altar_tier);
                    let cooldown = world.directors.first().map_or(0.0, |d| d.altar_cooldown_for_tier(selected_altar_tier));
                    fonts.draw_text(
                        &format!("T{}  {} souls  CD {:.1}s", selected_altar_tier, cost as u32, cooldown),
                        clamped.x + 34.0,
                        clamped.y - 18.0,
                        15.0,
                        GOLD,
                    );
                }
                PlacementMode::Wall => {
                    let can_place = can_place_wall(&world, clamped);
                    let border_color = if can_place { Color::new(0.4, 0.8, 1.0, 0.9) } else { Color::new(1.0, 0.2, 0.2, 0.9) };
                    let fill_color = if can_place { Color::new(0.2, 0.6, 0.9, 0.30) } else { Color::new(0.9, 0.2, 0.2, 0.30) };

                    draw_rectangle(clamped.x - 25.0, clamped.y - 25.0, 50.0, 50.0, fill_color);
                    draw_rectangle_lines(clamped.x - 25.0, clamped.y - 25.0, 50.0, 50.0, 2.2, border_color);

                    if let Some(tex) = textures.wall.as_ref() {
                        let tint = if can_place { Color::new(0.8, 0.9, 1.0, 0.75) } else { Color::new(1.0, 0.5, 0.5, 0.75) };
                        draw_texture_ex(tex, clamped.x - 25.0, clamped.y - 25.0, tint, DrawTextureParams {
                            dest_size: Some(Vec2::new(50.0, 50.0)),
                            ..Default::default()
                        });
                    }
                }
                PlacementMode::None => {}
            }
        }

        // 6. Отрисовка интерфейса на экране (screen-space)
        set_default_camera();
        let ui_interaction = match game_role {
            crate::render::main_menu::GameRole::Director => {
                render_ui(&world, &clock, &fonts, cam_pos, cam_zoom, &textures, audio.is_muted, audio.get_current_track_title())
            }
            crate::render::main_menu::GameRole::Boss => {
                crate::render::ui::render_boss_ui(&world, &clock, &fonts, audio.is_muted, audio.get_current_track_title())
            }
        };

        if net_driver.is_networked() {
            let (label, color) = if net_driver.is_network_host() {
                ("СЕТЬ · UDP ХОСТ".to_string(), Color::new(0.55, 0.95, 0.75, 0.95))
            } else if !net_driver.is_network_connected() {
                ("СЕТЬ · ПОДКЛЮЧЕНИЕ К ХОСТУ…".to_string(), Color::new(1.0, 0.75, 0.3, 0.95))
            } else {
                (format!("СЕТЬ · БОСС #{}", net_driver.assigned_player_id().unwrap_or(0)), Color::new(0.55, 0.85, 1.0, 0.95))
            };
            fonts.draw_heading(&label, 18.0, screen_h - 18.0, 12.0, color);
        }

        if placement_mode != PlacementMode::None {
            let banner_w = 490.0;
            let banner_h = 32.0;
            let banner_x = (screen_w - banner_w) * 0.5;
            let banner_y = 12.0;
            draw_rectangle(banner_x, banner_y, banner_w, banner_h, Color::new(0.08, 0.08, 0.16, 0.95));
            draw_rectangle_lines(banner_x, banner_y, banner_w, banner_h, 2.0, GOLD);
            let text = match placement_mode {
                PlacementMode::Altar => "УСТАНОВКА АЛТАРЯ: [ЛКМ] - Поставить | Ctrl+Q/W/E/R/A/S/D/F и Ctrl+Z/X/C/V/G/H/Y/U - тир | [Esc] - Отмена",
                PlacementMode::Wall => "УСТАНОВКА БАРРИКАДЫ: [ЛКМ] - Поставить  |  [ПКМ / Esc] - Отмена",
                PlacementMode::None => "",
            };
            fonts.draw_heading(text, banner_x + 16.0, banner_y + 21.0, 11.5, GOLD);
        }

        // Обработка интеракций из UI
        if ui_interaction.clicked_sound_toggle {
            audio.toggle_mute();
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }
        if ui_interaction.clicked_next_track {
            audio.next_track();
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }
        if ui_interaction.clicked_pause {
            is_paused = !is_paused;
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }
        if ui_interaction.clicked_codex {
            show_codex = !show_codex;
            if show_codex {
                active_dossier_tier = None;
                audio.handle_event(crate::audio::AudioEvent::BookOpen);
            } else {
                audio.handle_event(crate::audio::AudioEvent::BookClose);
            }
        }
        if let Some(target) = ui_interaction.map_click_world_pos {
            cam_pos = target;
            follow_boss = false;
        }
        if let Some(tier) = ui_interaction.open_dossier_tier {
            active_dossier_tier = Some(tier);
            opened_dossier_from_codex = false;
            audio.handle_event(crate::audio::AudioEvent::BookOpen);
        }
        if let Some(target) = ui_interaction.open_item_inspection {
            active_item_inspection = Some(target);
            opened_item_from_codex = false;
            audio.handle_event(crate::audio::AudioEvent::BookOpen);
        }
        if let Some(tier) = ui_interaction.direct_summon_tier {
            if let Some(director) = world.directors.first() {
                try_summon_squad(director, tier, &mut net_driver);
            }
        }
        if let Some(tier) = ui_interaction.direct_summon_max_tier {
            if let Some(director) = world.directors.first() {
                net_driver.send_command(Command::SpawnSquadMax { director_id: director.id, tier });
            }
        }
        if let Some(grade) = ui_interaction.upgrade_grade {
            if let Some(director) = world.directors.first() {
                net_driver.send_command(Command::UpgradeDirectorGrade { director_id: director.id, grade_type: grade });
            }
        }
        if let Some(grade) = ui_interaction.upgrade_grade_max {
            if let Some(director) = world.directors.first() {
                net_driver.send_command(Command::UpgradeDirectorGradeMax { director_id: director.id, grade_type: grade });
            }
        }
        if let Some(page) = ui_interaction.set_mob_page {
            if let Some(director) = world.directors.first() {
                net_driver.send_command(Command::SetMobPage { director_id: director.id, page });
            }
        }

        if let Some(tier) = ui_interaction.altar_tier {
            selected_altar_tier = tier;
        }
        if ui_interaction.start_altar_placement {
            placement_mode = PlacementMode::Altar;
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }
        if ui_interaction.start_wall_placement || is_key_pressed(KeyCode::B) {
            placement_mode = PlacementMode::Wall;
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }

        // Логика перемещения карты или подтверждения установки Структуры
        if placement_mode != PlacementMode::None {
            if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                placement_mode = PlacementMode::None;
                audio.handle_event(crate::audio::AudioEvent::UiClick);
            } else if is_mouse_button_pressed(MouseButton::Left) && !ui_interaction.is_mouse_over_ui {
                let (mx, my) = mouse_position();
                let world_click = world_cam.screen_to_world(Vec2::new(mx, my));
                let clamped_click = snap_to_build_grid(world_click);

                match placement_mode {
                    PlacementMode::Altar => {
                        let cost = altar_cost(selected_altar_tier);
                        let altar_type = structure_type_for_altar_tier(selected_altar_tier);
                        if can_place_blood_altar(&world, clamped_click) {
                            if let Some(director) = world.directors.first() {
                                if director.points >= cost && director.altar_cooldown_for_tier(selected_altar_tier) <= 0.0 {
                                    net_driver.send_command(Command::PlaceStructure {
                                        director_id: director.id,
                                        structure_type: altar_type,
                                        pos: clamped_click.into(),
                                    });
                                    placement_mode = PlacementMode::None;
                                }
                            }
                        } else {
                            world.add_floating_text(clamped_click, "Нельзя построить здесь!".to_string(), RED, 1.2);
                        }
                    }
                    PlacementMode::Wall => {
                        if can_place_wall(&world, clamped_click) {
                            if let Some(director) = world.directors.first() {
                                if director.points >= WALL_COST && director.wall_cooldown <= 0.0 {
                                    net_driver.send_command(Command::PlaceStructure {
                                        director_id: director.id,
                                        structure_type: STRUCTURE_WALL,
                                        pos: clamped_click.into(),
                                    });
                                    placement_mode = PlacementMode::None;
                                }
                            }
                        } else {
                            world.add_floating_text(clamped_click, "Нельзя построить здесь!".to_string(), RED, 1.2);
                        }
                    }
                    PlacementMode::None => {}
                }
            }
        } else {
            if is_mouse_button_pressed(MouseButton::Left) {
                if !ui_interaction.is_mouse_over_ui && !show_codex && active_dossier_tier.is_none() && active_item_inspection.is_none() && !is_paused && !is_game_over {
                    is_dragging_map = true;
                }
            }
            if is_mouse_button_down(MouseButton::Left) && is_dragging_map {
                cam_pos -= mouse_delta / cam_zoom;
                cam_pos.x = cam_pos.x.clamp(200.0, MAP_WIDTH - 200.0);
                cam_pos.y = cam_pos.y.clamp(200.0, MAP_HEIGHT - 200.0);
                follow_boss = false;
            }
            if is_mouse_button_released(MouseButton::Left) {
                is_dragging_map = false;
            }
        }
        last_mouse_pos = current_mpos;

        // 7. Отрисовка модальных окон (Кодекс, Вики-досье, Осмотр предметов/рецептов, Пауза или Завершение игры)
        if let Some(target) = active_item_inspection {
            let close = crate::render::item_dossier::draw_item_inspection_modal(target, &world, &fonts, &textures);
            if close {
                active_item_inspection = None;
                audio.handle_event(crate::audio::AudioEvent::BookClose);
                if opened_item_from_codex {
                    show_codex = true;
                    opened_item_from_codex = false;
                }
            }
        } else if show_codex {
            if let Some(res) = draw_codex(&mut codex_tab, &fonts, &textures) {
                show_codex = false;
                match res {
                    crate::render::codex::CodexResult::MobDossier(tier) => {
                        active_dossier_tier = Some(tier);
                        opened_dossier_from_codex = true;
                        audio.handle_event(crate::audio::AudioEvent::BookOpen);
                    }
                    crate::render::codex::CodexResult::ItemInspection(target) => {
                        active_item_inspection = Some(target);
                        opened_item_from_codex = true;
                        audio.handle_event(crate::audio::AudioEvent::BookOpen);
                    }
                }
            }
        } else if let Some(tier) = active_dossier_tier {
            let res = draw_mob_dossier(tier, &world, &fonts, &textures);
            if res.close_requested {
                active_dossier_tier = None;
                audio.handle_event(crate::audio::AudioEvent::BookClose);
                if opened_dossier_from_codex {
                    show_codex = true;
                    opened_dossier_from_codex = false;
                }
            }
            if res.summon_requested {
                if let Some(director) = world.directors.first() {
                    try_summon_squad(director, tier, &mut net_driver);
                }
            }
        } else if is_paused && !is_game_over {
            let action = crate::render::settings::draw_pause_settings(&fonts, &mut audio);
            match action {
                crate::render::settings::SettingsAction::Resume => {
                    is_paused = false;
                }
                crate::render::settings::SettingsAction::Restart => {
                    let duration_secs = clock.duration_secs;
                    world = World::new();
                    world.boss_model = boss_model.clone();
                    world.match_duration_secs = duration_secs;
                    apply_player_profile(&mut world, &menu.settings, game_role);
                    clock = GameClock::with_duration(duration_secs);
                    cam_pos = world.boss_pos;
                    follow_boss = true;
                    spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
                    spawn_corner_obelisks(&mut world);
                    is_paused = false;
                    placement_mode = PlacementMode::None;
                    show_codex = false;
                    active_dossier_tier = None;
                    active_item_inspection = None;
                    opened_item_from_codex = false;
                    opened_dossier_from_codex = false;
                    boss_death_sound_played = false;
                    audio.shuffle_start();
                }
                crate::render::settings::SettingsAction::OpenCodex => {
                    is_paused = false;
                    show_codex = true;
                    active_dossier_tier = None;
                    active_item_inspection = None;
                }
                crate::render::settings::SettingsAction::None => {}
            }
        } else if is_game_over {
            let victory = !world.any_boss_alive();
            draw_game_over(&world, &clock, victory, &fonts);
            if is_key_pressed(KeyCode::Space) && !net_driver.is_network_client() {
                let duration_secs = clock.duration_secs;
                world = World::new();
                world.boss_model = boss_model.clone();
                world.match_duration_secs = duration_secs;
                apply_player_profile(&mut world, &menu.settings, game_role);
                clock = GameClock::with_duration(duration_secs);
                cam_pos = world.boss_pos;
                follow_boss = true;
                spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
                spawn_corner_obelisks(&mut world);
                for command in net_driver.rejoin_commands() {
                    execute_command(&mut world, command);
                }
                is_paused = false;
                placement_mode = PlacementMode::None;
                show_codex = false;
                active_dossier_tier = None;
                active_item_inspection = None;
                opened_item_from_codex = false;
                opened_dossier_from_codex = false;
                boss_death_sound_played = false;
                audio.shuffle_start();
            } else if is_key_pressed(KeyCode::Escape) {
                show_main_menu = true;
                show_mouse(true);
            }
        }

        // 8. Отрисовка тематичного курсора Повелителя поверх всех окон
        let is_hovering = ui_interaction.is_mouse_over_ui || show_codex || active_dossier_tier.is_some() || active_item_inspection.is_some() || is_paused;
        draw_overlord_cursor(
            current_mpos,
            is_mouse_button_down(MouseButton::Left),
            is_hovering,
            get_time() as f32,
        );

        // Авто-скриншот после отрисовки кадра
        if auto_screenshot && frame_count == 60 {
            if auto_settings {
                get_screen_data().export_png("screenshot_settings.png");
            } else if auto_dossier {
                get_screen_data().export_png("screenshot_dossier.png");
            } else if auto_recipe {
                get_screen_data().export_png("screenshot_recipe.png");
            } else if auto_codex {
                get_screen_data().export_png("screenshot_codex.png");
            } else {
                get_screen_data().export_png("screenshot.png");
            }
            break;
        }

        next_frame().await;
    }
}

fn handle_camera_input(
    cam_pos: &mut Vec2,
    cam_zoom: &mut f32,
    follow_boss: &mut bool,
    boss_pos: Vec2,
    dt: f32,
    arrows_only: bool,
) {
    let mut move_dir = Vec2::ZERO;

    if is_key_down(KeyCode::Up) || (!arrows_only && is_key_down(KeyCode::W)) {
        move_dir.y -= 1.0;
    }
    if is_key_down(KeyCode::Down) || (!arrows_only && is_key_down(KeyCode::S)) {
        move_dir.y += 1.0;
    }
    if is_key_down(KeyCode::Left) || (!arrows_only && is_key_down(KeyCode::A)) {
        move_dir.x -= 1.0;
    }
    if is_key_down(KeyCode::Right) || (!arrows_only && is_key_down(KeyCode::D)) {
        move_dir.x += 1.0;
    }

    if move_dir.length_squared() > 0.0 {
        *follow_boss = false;
        let speed = if is_key_down(KeyCode::LeftShift) { 1600.0 } else { 850.0 };
        *cam_pos += move_dir.normalize() * (speed * dt / *cam_zoom);
    }

    // Центрирование камеры на боссе по клавише C
    if is_key_pressed(KeyCode::C) {
        *follow_boss = true;
    }

    if *follow_boss {
        *cam_pos = cam_pos.lerp(boss_pos, (dt * 7.0).min(1.0));
    }

    // Масштабирование колесиком мыши
    let wheel = mouse_wheel().1;
    if wheel > 0.0 {
        *cam_zoom = (*cam_zoom * 1.15).min(2.0);
    } else if wheel < 0.0 {
        *cam_zoom = (*cam_zoom / 1.15).max(0.4);
    }

    // Ограничение камеры размерами карты
    cam_pos.x = cam_pos.x.clamp(200.0, MAP_WIDTH - 200.0);
    cam_pos.y = cam_pos.y.clamp(200.0, MAP_HEIGHT - 200.0);
}

fn draw_build_grid_overlay(cam_pos: Vec2, cam_zoom: f32, screen_w: f32, screen_h: f32) {
    const CELL: f32 = crate::content::structures::BUILD_GRID_SIZE;
    let half_w = screen_w / (2.0 * cam_zoom.max(0.15));
    let half_h = screen_h / (2.0 * cam_zoom.max(0.15));
    let min_x = (cam_pos.x - half_w).max(0.0);
    let max_x = (cam_pos.x + half_w).min(MAP_WIDTH);
    let min_y = (cam_pos.y - half_h).max(0.0);
    let max_y = (cam_pos.y + half_h).min(MAP_HEIGHT);
    let grid_color = Color::new(0.45, 0.72, 0.92, 0.18);

    let mut x = (min_x / CELL).floor() * CELL;
    while x <= max_x {
        draw_line(x, min_y, x, max_y, 0.7, grid_color);
        x += CELL;
    }
    let mut y = (min_y / CELL).floor() * CELL;
    while y <= max_y {
        draw_line(min_x, y, max_x, y, 0.7, grid_color);
        y += CELL;
    }
}

fn collect_boss_player_input(
    settings: &crate::render::main_menu::PlayerSettings,
    world: &World,
    world_cam: &Camera2D,
) -> (Vec2, Vec2, bool) {
    use crate::render::main_menu::PlayerControl;

    let mut move_dir = Vec2::ZERO;
    if is_key_down(settings.key_for(PlayerControl::Up)) {
        move_dir.y -= 1.0;
    }
    if is_key_down(settings.key_for(PlayerControl::Down)) {
        move_dir.y += 1.0;
    }
    if is_key_down(settings.key_for(PlayerControl::Left)) {
        move_dir.x -= 1.0;
    }
    if is_key_down(settings.key_for(PlayerControl::Right)) {
        move_dir.x += 1.0;
    }
    if move_dir.length_squared() > 1.0 {
        move_dir = move_dir.normalize();
    }
    let (mx, my) = mouse_position();
    let mouse_world = world_cam.screen_to_world(Vec2::new(mx, my));
    let aim_dir = (mouse_world - world.boss_pos).normalize_or_zero();
    let dash_pressed = is_key_pressed(settings.key_for(PlayerControl::Dash));
    (move_dir, aim_dir, dash_pressed)
}

fn apply_player_profile(
    world: &mut World,
    settings: &crate::render::main_menu::PlayerSettings,
    role: crate::render::main_menu::GameRole,
) {
    world.boss_name = match role {
        crate::render::main_menu::GameRole::Director => "Соло-Босс".to_string(),
        crate::render::main_menu::GameRole::Boss => settings.nickname.chars().take(32).collect(),
    };
    world.boss_skin = settings.skin % 5;
}

fn handle_player_input(world: &World, driver: &mut impl NetworkDriver, _world_cam: &Camera2D) {
    if let Some(director) = world.directors.first() {
        let is_shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let is_ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);

        // Переключение страницы мобов Директора: клавиши [1], [2] и [Tab]
        if is_key_pressed(KeyCode::Tab) {
            let next_page = if director.mob_page == 0 { 1 } else { 0 };
            driver.send_command(Command::SetMobPage { director_id: director.id, page: next_page });
        }
        if is_key_pressed(KeyCode::Key1) {
            driver.send_command(Command::SetMobPage { director_id: director.id, page: 0 });
        }
        if is_key_pressed(KeyCode::Key2) {
            driver.send_command(Command::SetMobPage { director_id: director.id, page: 1 });
        }

        if !is_ctrl {
            // 1. Горячие клавиши Q, W, E, R, A, S, D, F для спавна отрядов
            // (Shift = спавн на все деньги)
            let keys = [
                KeyCode::Q, KeyCode::W, KeyCode::E, KeyCode::R,
                KeyCode::A, KeyCode::S, KeyCode::D, KeyCode::F,
            ];
            for (i, key) in keys.iter().enumerate() {
                if is_key_pressed(*key) {
                    let tier = (i + 1) as u8 + (director.mob_page * 8);
                    if is_shift {
                        driver.send_command(Command::SpawnSquadMax { director_id: director.id, tier });
                    } else {
                        try_summon_squad(director, tier, driver);
                    }
                }
            }

            // 1.1 Горячие клавиши Z, X, C для улучшений Директора.
            if is_key_pressed(KeyCode::Z) {
                if is_shift {
                    driver.send_command(Command::UpgradeDirectorGradeMax { director_id: director.id, grade_type: 0 });
                } else {
                    driver.send_command(Command::UpgradeDirectorGrade { director_id: director.id, grade_type: 0 });
                }
            }
            if is_key_pressed(KeyCode::X) {
                if is_shift {
                    driver.send_command(Command::UpgradeDirectorGradeMax { director_id: director.id, grade_type: 1 });
                } else {
                    driver.send_command(Command::UpgradeDirectorGrade { director_id: director.id, grade_type: 1 });
                }
            }
            if is_key_pressed(KeyCode::C) {
                if is_shift {
                    driver.send_command(Command::UpgradeDirectorGradeMax { director_id: director.id, grade_type: 2 });
                } else {
                    driver.send_command(Command::UpgradeDirectorGrade { director_id: director.id, grade_type: 2 });
                }
            }
        }
    }
}

fn try_summon_squad(director: &crate::ecs::world::DirectorState, tier: u8, driver: &mut impl NetworkDriver) {
    let config = get_squad_config(tier);
    let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;

    // Кулдаун снят: при наличии очков душ призывать можно непрерывно!
    if is_unlocked && director.points >= config.cost {
        driver.send_command(Command::SpawnSquad {
            director_id: director.id,
            tier,
        });
    }
}

fn execute_command(world: &mut World, cmd: Command) {
    match cmd {
        Command::SpawnSquad { director_id, tier } => {
            let config = get_squad_config(tier);
            let can_spawn = if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                director.spend_points(config.cost)
            } else {
                false
            };
            if can_spawn {
                spawn_squad_perimeter(world, tier);
                if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                    director.time_since_last_spawn = 0.0;
                }
                world.push_audio(crate::audio::AudioEvent::SummonMob);
            }
        }
        Command::SpawnSquadMax { director_id, tier } => {
            let config = get_squad_config(tier);
            let mut count = 0;
            if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
                if is_unlocked {
                    while director.spend_points(config.cost) && count < 50 {
                        count += 1;
                    }
                }
            }
            for _ in 0..count {
                spawn_squad_perimeter(world, tier);
            }
            if count > 0 {
                if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                    director.time_since_last_spawn = 0.0;
                }
                world.push_audio(crate::audio::AudioEvent::SummonMob);
                world.add_floating_text(
                    world.boss_pos,
                    format!("⚡ ПРИЗЫВ НА ВСЕ: +{} отрядов {}!", count, config.name),
                    GOLD,
                    1.5,
                );
            }
        }
        Command::UpgradeDirectorGrade { director_id, grade_type } => {
            let mut notification = None;
            let mut carapace_upgraded = false;
            if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                match grade_type {
                    0 => {
                        let cost = 100.0 * (director.grade_fury as f32 + 1.0);
                        if director.spend_points(cost) {
                            director.grade_fury += 1;
                            notification = Some((format!("🔥 ЯРОСТЬ КРОВИ (Ур. {})", director.grade_fury), RED));
                        }
                    }
                    1 => {
                        let cost = 120.0 * (director.grade_carapace as f32 + 1.0);
                        if director.spend_points(cost) {
                            director.grade_carapace += 1;
                            carapace_upgraded = true;
                            notification = Some((format!("🛡️ ПАНЦИРЬ (Ур. {})", director.grade_carapace), SKYBLUE));
                        }
                    }
                    2 => {
                        let cost = 150.0 * (director.grade_pestilence as f32 + 1.0);
                        if director.spend_points(cost) {
                            director.grade_pestilence += 1;
                            notification = Some((format!("🧪 ЧУМНЫЕ СПОРЫ (Ур. {})", director.grade_pestilence), LIME));
                        }
                    }
                    _ => {}
                }
            }
            if carapace_upgraded {
                crate::content::structures::upgrade_structure_carapace(world, 1);
            }
            if let Some((text, col)) = notification {
                world.add_floating_text(world.boss_pos, text, col, 1.4);
                world.push_audio(crate::audio::AudioEvent::UpgradeGrade);
            }
        }
        Command::UpgradeDirectorGradeMax { director_id, grade_type } => {
            let mut notification = None;
            let mut carapace_upgrades = 0u32;
            if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                let mut upgraded = 0;
                match grade_type {
                    0 => {
                        while upgraded < 1000 {
                            let cost = 100.0 * (director.grade_fury as f32 + 1.0);
                            if director.spend_points(cost) {
                                director.grade_fury += 1;
                                upgraded += 1;
                            } else {
                                break;
                            }
                        }
                        if upgraded > 0 {
                            notification = Some((format!("🔥 ЯРОСТЬ КРОВИ (+{} ур. -> Ур. {})", upgraded, director.grade_fury), RED));
                        }
                    }
                    1 => {
                        while upgraded < 1000 {
                            let cost = 120.0 * (director.grade_carapace as f32 + 1.0);
                            if director.spend_points(cost) {
                                director.grade_carapace += 1;
                                upgraded += 1;
                            } else {
                                break;
                            }
                        }
                        carapace_upgrades = upgraded;
                        if upgraded > 0 {
                            notification = Some((format!("🛡️ ПАНЦИРЬ (+{} ур. -> Ур. {})", upgraded, director.grade_carapace), SKYBLUE));
                        }
                    }
                    2 => {
                        while upgraded < 1000 {
                            let cost = 150.0 * (director.grade_pestilence as f32 + 1.0);
                            if director.spend_points(cost) {
                                director.grade_pestilence += 1;
                                upgraded += 1;
                            } else {
                                break;
                            }
                        }
                        if upgraded > 0 {
                            notification = Some((format!("🧪 ЧУМНЫЕ СПОРЫ (+{} ур. -> Ур. {})", upgraded, director.grade_pestilence), LIME));
                        }
                    }
                    _ => {}
                }
            }
            crate::content::structures::upgrade_structure_carapace(world, carapace_upgrades);
            if let Some((text, col)) = notification {
                world.add_floating_text(world.boss_pos, text, col, 1.5);
                world.push_audio(crate::audio::AudioEvent::UpgradeGrade);
            }
        }
        Command::PlaceStructure {
            director_id,
            structure_type,
            pos,
        } => {
            match structure_type {
                STRUCTURE_WALL => {
                    let clamped_pos = snap_to_build_grid(pos.into());
                    let mut can_build = false;
                    if can_place_wall(world, clamped_pos) {
                        if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                            if director.wall_cooldown <= 0.0 && director.spend_points(WALL_COST) {
                                director.wall_cooldown = director.wall_max_cooldown;
                                can_build = true;
                            }
                        }
                    }
                    if can_build {
                        try_spawn_wall(world, clamped_pos);
                        world.push_audio(crate::audio::AudioEvent::AltarBuild);
                    }
                }
                altar_type if altar_tier_from_structure_type(altar_type).is_some() => {
                    let tier = altar_tier_from_structure_type(altar_type).unwrap_or(1);
                    let cost = altar_cost(tier);
                    let cooldown = altar_cooldown(tier);
                    let clamped_pos = snap_to_build_grid(pos.into());
                    let mut can_build = false;
                    if can_place_blood_altar(world, clamped_pos) {
                        if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                            if director.altar_cooldown_for_tier(tier) <= 0.0
                                && director.spend_points(cost)
                            {
                                director.set_altar_cooldown_for_tier(tier, cooldown);
                                can_build = true;
                            }
                        }
                    }
                    if can_build {
                        try_spawn_tier_altar(world, clamped_pos, tier);
                        world.push_audio(crate::audio::AudioEvent::AltarBuild);
                    }
                }
                _ => {}
            }
        }
        Command::JoinBoss { player_id, name, skin } => {
            if player_id == 1 {
                world.boss_name = name.chars().take(32).collect();
                world.boss_skin = skin % 5;
                world.boss_human_controlled = true;
            } else {
                let angle = player_id.saturating_sub(2) as f32 * std::f32::consts::TAU / 7.0;
                let spawn_pos = Vec2::new(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5)
                    + Vec2::new(angle.cos(), angle.sin()) * 150.0;
                world.add_coop_boss(player_id, spawn_pos, name, skin);
                if let Some(player) = world.coop_bosses.iter_mut().find(|player| player.id == player_id) {
                    player.human_controlled = true;
                }
            }
        }
        Command::BossInput { player_id, move_dir, aim_dir, dash_pressed } => {
            let move_dir: Vec2 = move_dir.into();
            let aim_dir: Vec2 = aim_dir.into();
            let move_dir = if move_dir.is_finite() { move_dir.clamp_length_max(1.0) } else { Vec2::ZERO };
            if player_id == 1 {
                world.boss_input_move_dir = move_dir;
                world.boss_input_aim_dir = if aim_dir.is_finite() && aim_dir.length_squared() > 0.01 {
                    aim_dir.normalize()
                } else { world.boss_facing };
                world.boss_input_dash_pressed |= dash_pressed;
            } else if let Some(player) = world.coop_bosses.iter_mut().find(|player| player.id == player_id) {
                player.input_move_dir = move_dir;
                player.input_aim_dir = if aim_dir.is_finite() && aim_dir.length_squared() > 0.01 {
                    aim_dir.normalize()
                } else { player.boss_facing };
                player.input_dash_pressed |= dash_pressed;
            }
        }
        Command::DisconnectBoss { player_id } => {
            if player_id == 1 {
                world.boss_human_controlled = false;
                world.boss_input_move_dir = Vec2::ZERO;
                world.boss_input_dash_pressed = false;
            } else if let Some(player) = world.coop_bosses.iter_mut().find(|player| player.id == player_id) {
                player.human_controlled = false;
                player.input_move_dir = Vec2::ZERO;
                player.input_dash_pressed = false;
            }
        }
        Command::SetMobPage { director_id, page } => {
            if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                director.mob_page = page;
            }
        }
        _ => {}
    }
}

fn draw_game_over(world: &World, clock: &GameClock, victory: bool, fonts: &GameFonts) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Затемнение фона
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.85));

    let title = if victory {
        "ПОБЕДА ДИРЕКТОРА ЗЛА! БОСС УНИЧТОЖЕН!"
    } else {
        "ПОРАЖЕНИЕ! БОСС ВЫЖИЛ 17 МИНУТ!"
    };
    let title_color = if victory { GREEN } else { RED };
    let title_font_size = 44.0;
    let tw = fonts.measure_text(title, title_font_size).width;
    fonts.draw_text(title, (screen_w - tw) / 2.0, screen_h * 0.25, title_font_size, title_color);

    // Статистика сражения
    let director = world.directors.first();
    let total_spawned = director.map(|d| d.total_mobs_spawned).unwrap_or(0);
    let total_spent = director.map(|d| d.total_points_spent).unwrap_or(0.0);
    let final_income = director.map(|d| d.income_rate).unwrap_or(0.0);

    let stats = [
        format!("Время выживания босса: {}", clock.formatted_elapsed()),
        format!("Финальный уровень босса: {}", world.boss_inventory.level),
        format!("Суммарный нанесенный урон: {:.0}", world.total_damage_dealt_to_boss),
        format!("Призвано монстров: {}", total_spawned),
        format!("Потрачено очков душ: {:.0}", total_spent),
        format!("Финальный доход: +{:.1} каждые 2 сек", final_income),
    ];

    let start_y = screen_h * 0.36;
    for (i, stat) in stats.iter().enumerate() {
        let sw = fonts.measure_text(stat, 20.0).width;
        fonts.draw_text(stat, (screen_w - sw) / 2.0, start_y + (i as f32) * 34.0, 20.0, WHITE);
    }

    // Подсказка перезапуска
    let restart_prompt = "Нажмите [ Пробел ] для перезапуска сражения | [ Esc ] в меню";
    let rw = fonts.measure_text(restart_prompt, 22.0).width;
    fonts.draw_text(restart_prompt, (screen_w - rw) / 2.0, screen_h * 0.78, 22.0, GOLD);
}
