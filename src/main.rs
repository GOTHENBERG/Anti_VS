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
use crate::content::structures::{can_place_blood_altar, spawn_blood_altar, try_spawn_blood_altar, BLOOD_ALTAR_COST};
use crate::core::time::GameClock;
use crate::core::{MAP_HEIGHT, MAP_WIDTH};
use crate::ecs::systems::{
    boss_ai::update_boss_ai,
    combat::update_combat,
    director::update_director,
    meditation::update_meditation,
    movement::update_movement,
    skills::update_skills,
};
use crate::ecs::world::World;
use crate::network::driver::{LocalDriver, NetworkDriver};
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
    // Поддерживает аргументы: cargo run --release -- --train [поколений] [размер популяции] [300 / 1020] [--cap 300|1020] [--clean / --reset]
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--train") {
        let generations = args.get(pos + 1).and_then(|s| s.parse().ok()).unwrap_or(8);
        let population = args.get(pos + 2).and_then(|s| s.parse().ok()).unwrap_or(8);
        let reset = args.iter().any(|a| a == "--clean" || a == "--reset");

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

        crate::ai::trainer::run_co_evolutionary_training(generations, population, reset, round_duration);
        return;
    }

    let boss_model = crate::ai::load_trained_boss_model();
    let mut world = World::new();
    // Модель загружается один раз за сессию, а не на каждом создании мира.
    world.boss_model = boss_model.clone();
    let mut clock = GameClock::new();
    let mut net_driver = LocalDriver::new();
    let fonts = GameFonts::load();
    let textures = GameTextures::load().await;
    let mut audio = AudioManager::load().await;
    let auto_screenshot = std::env::args().any(|a| a == "--screenshot");
    let auto_codex = std::env::args().any(|a| a == "--codex");
    let auto_dossier = std::env::args().any(|a| a == "--dossier");
    let auto_recipe = std::env::args().any(|a| a == "--recipe" || a == "--dossier-item");
    let auto_settings = std::env::args().any(|a| a == "--settings" || a == "--pause");
    let mut is_paused = auto_settings;
    let mut boss_death_sound_played = false;

    // Скрываем стандартный системный курсор мыши (заменяем демонической дланью Повелителя)
    show_mouse(false);

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
    let mut is_placing_altar = false;
    let mut active_dossier_tier: Option<u8> = if auto_dossier { Some(1) } else { None };
    let mut opened_dossier_from_codex = false;
    let mut active_item_inspection: Option<crate::render::item_dossier::ItemInspectionTarget> = if auto_recipe {
        Some(crate::render::item_dossier::ItemInspectionTarget::Weapon(crate::ecs::components::inventory::WeaponType::Hellfire, 8, 0))
    } else {
        None
    };
    let mut opened_item_from_codex = false;

    // Начальный разрушаемый алтарь игрока
    spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));

    // Запуск динамичного джазового / симфонического плейлиста (со случайного трека)
    if !auto_screenshot && !auto_codex && !auto_dossier && !auto_recipe && !auto_settings {
        audio.shuffle_start();
    }

    loop {
        let dt = get_frame_time().min(0.05);
        frame_count += 1;
        let current_mpos = Vec2::from(mouse_position());
        let mouse_delta = current_mpos - last_mouse_pos;

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

        // Пауза по Space или P
        if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Space) {
            is_paused = !is_paused;
            audio.handle_event(crate::audio::AudioEvent::UiClick);
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
        let is_game_over = !world.boss_health.is_alive() || clock.is_time_up();
        if !world.boss_health.is_alive() && !boss_death_sound_played {
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

        if is_key_pressed(KeyCode::R) {
            world = World::new();
            world.boss_model = boss_model.clone();
            clock = GameClock::new();
            cam_pos = world.boss_pos;
            follow_boss = true;
            spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
            is_paused = false;
            is_placing_altar = false;
            show_codex = false;
            active_dossier_tier = None;
            active_item_inspection = None;
            opened_item_from_codex = false;
            opened_dossier_from_codex = false;
            boss_death_sound_played = false;
            audio.shuffle_start();
        }

        // 1. Управление камерой на большой карте: WASD + колесо мыши + центрирование по C
        handle_camera_input(&mut cam_pos, &mut cam_zoom, &mut follow_boss, world.boss_pos, dt);

        let screen_w = screen_width();
        let screen_h = screen_height();
        let world_cam = Camera2D {
            target: cam_pos,
            zoom: Vec2::new(cam_zoom / (screen_w / 2.0), cam_zoom / (screen_h / 2.0)),
            ..Default::default()
        };

        let is_running = !is_game_over && !is_paused && !show_codex && active_dossier_tier.is_none();

        // 2. Сбор ввода игрока-директора -> генерация Команд
        if is_running {
            clock.tick(dt);
            handle_player_input(&world, &mut net_driver, &world_cam);
        }

        // 3. Исполнение сетевых/локальных команд (Command Pattern)
        let commands = net_driver.poll_commands();
        for cmd in commands {
            execute_command(&mut world, cmd);
        }

        // 4. Обновление систем мира
        if is_running {
            update_director(&mut world, dt);
            update_boss_ai(&mut world, dt);
            update_meditation(&mut world, dt);
            update_skills(&mut world, dt);
            update_movement(&mut world, dt);
            update_combat(&mut world, dt);
            audio.process_events(&mut world);
            audio.update_music(dt, cam_pos, world.boss_pos);
        } else {
            audio.update_music(dt, cam_pos, world.boss_pos);
        }

        // 5. Отрисовка мира через камеру с Viewport Frustum Culling
        set_camera(&world_cam);
        render_world(&world, &fonts, &textures, cam_pos, cam_zoom);

        // Призрак Алтаря при интерактивном режиме установки
        if is_placing_altar {
            let (mx, my) = mouse_position();
            let world_cursor = world_cam.screen_to_world(Vec2::new(mx, my));
            let margin = 45.0;
            let clamped = Vec2::new(
                world_cursor.x.clamp(margin, MAP_WIDTH - margin),
                world_cursor.y.clamp(margin, MAP_HEIGHT - margin),
            );
            let can_place = can_place_blood_altar(&world, clamped);
            let border_color = if can_place {
                Color::new(0.2, 1.0, 0.4, 0.9)
            } else {
                Color::new(1.0, 0.2, 0.2, 0.9)
            };
            let fill_color = if can_place {
                Color::new(0.2, 0.9, 0.4, 0.25)
            } else {
                Color::new(0.9, 0.2, 0.2, 0.25)
            };

            // Радиус дистанции между алтарями (ALTAR_MIN_SPACING = 120.0)
            draw_circle_lines(clamped.x, clamped.y, 120.0, 1.5, Color::new(border_color.r, border_color.g, border_color.b, 0.35));
            // Радиус самого алтаря
            draw_circle(clamped.x, clamped.y, 30.0, fill_color);
            draw_circle_lines(clamped.x, clamped.y, 30.0, 2.5, border_color);

            if let Some(tex) = textures.altar.as_ref() {
                let w = 48.0;
                let h = 70.0;
                let tint = if can_place {
                    Color::new(0.8, 1.0, 0.8, 0.75)
                } else {
                    Color::new(1.0, 0.5, 0.5, 0.75)
                };
                draw_texture_ex(
                    tex,
                    clamped.x - w * 0.5,
                    clamped.y - h * 0.5,
                    tint,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(w, h)),
                        ..Default::default()
                    },
                );
            }
        }

        // 6. Отрисовка интерфейса на экране (screen-space)
        set_default_camera();
        let ui_interaction = render_ui(&world, &clock, &fonts, cam_pos, cam_zoom, &textures, audio.is_muted, audio.get_current_track_title());

        if is_placing_altar {
            let banner_w = 460.0;
            let banner_h = 32.0;
            let banner_x = (screen_w - banner_w) * 0.5;
            let banner_y = 12.0;
            draw_rectangle(banner_x, banner_y, banner_w, banner_h, Color::new(0.08, 0.08, 0.16, 0.95));
            draw_rectangle_lines(banner_x, banner_y, banner_w, banner_h, 2.0, GOLD);
            fonts.draw_heading("УСТАНОВКА: [ЛКМ] - Поставить  |  [ПКМ / Esc] - Отмена", banner_x + 16.0, banner_y + 21.0, 12.0, GOLD);
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

        if ui_interaction.start_altar_placement {
            is_placing_altar = true;
            audio.handle_event(crate::audio::AudioEvent::UiClick);
        }

        // Логика перемещения карты или подтверждения установки Алтаря
        if is_placing_altar {
            if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                is_placing_altar = false;
                audio.handle_event(crate::audio::AudioEvent::UiClick);
            } else if is_mouse_button_pressed(MouseButton::Left) && !ui_interaction.is_mouse_over_ui {
                let (mx, my) = mouse_position();
                let world_click = world_cam.screen_to_world(Vec2::new(mx, my));
                let margin = 45.0;
                let clamped_click = Vec2::new(
                    world_click.x.clamp(margin, MAP_WIDTH - margin),
                    world_click.y.clamp(margin, MAP_HEIGHT - margin),
                );
                if can_place_blood_altar(&world, clamped_click) {
                    if let Some(director) = world.directors.first() {
                        if director.points >= BLOOD_ALTAR_COST && director.altar_cooldown <= 0.0 {
                            net_driver.send_command(Command::PlaceStructure {
                                director_id: director.id,
                                structure_type: 1,
                                pos: clamped_click.into(),
                            });
                            is_placing_altar = false;
                        }
                    }
                } else {
                    world.add_floating_text(clamped_click, "Нельзя построить здесь!".to_string(), RED, 1.2);
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
                    world = World::new();
                    world.boss_model = boss_model.clone();
                    clock = GameClock::new();
                    cam_pos = world.boss_pos;
                    follow_boss = true;
                    spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
                    is_paused = false;
                    is_placing_altar = false;
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
            let victory = !world.boss_health.is_alive();
            draw_game_over(&world, &clock, victory, &fonts);
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
) {
    let mut move_dir = Vec2::ZERO;

    if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
        move_dir.y -= 1.0;
    }
    if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
        move_dir.y += 1.0;
    }
    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        move_dir.x -= 1.0;
    }
    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
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

fn handle_player_input(world: &World, driver: &mut impl NetworkDriver, _world_cam: &Camera2D) {
    if let Some(director) = world.directors.first() {
        let is_shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

        // Переключение страницы мобов Директора по клавише [Tab] (Страница 1: 1-8, Страница 2: 9-16)
        if is_key_pressed(KeyCode::Tab) {
            let next_page = if director.mob_page == 0 { 1 } else { 0 };
            driver.send_command(Command::SetMobPage { director_id: director.id, page: next_page });
        }

        // 1. Горячие клавиши 1, 2, 3, 4, 5, 6, 7, 8 для спавна отрядов (с поддержкой Shift для спавна на все деньги)
        let keys = [
            KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
            KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8,
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

        // 1.1 Горячие клавиши F, G, H для улучшений мобов Директора (с поддержкой Shift для прокачки на все деньги)
        if is_key_pressed(KeyCode::F) {
            if is_shift {
                driver.send_command(Command::UpgradeDirectorGradeMax { director_id: director.id, grade_type: 0 });
            } else {
                driver.send_command(Command::UpgradeDirectorGrade { director_id: director.id, grade_type: 0 });
            }
        }
        if is_key_pressed(KeyCode::G) {
            if is_shift {
                driver.send_command(Command::UpgradeDirectorGradeMax { director_id: director.id, grade_type: 1 });
            } else {
                driver.send_command(Command::UpgradeDirectorGrade { director_id: director.id, grade_type: 1 });
            }
        }
        if is_key_pressed(KeyCode::H) {
            if is_shift {
                driver.send_command(Command::UpgradeDirectorGradeMax { director_id: director.id, grade_type: 2 });
            } else {
                driver.send_command(Command::UpgradeDirectorGrade { director_id: director.id, grade_type: 2 });
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
            if let Some((text, col)) = notification {
                world.add_floating_text(world.boss_pos, text, col, 1.4);
                world.push_audio(crate::audio::AudioEvent::UpgradeGrade);
            }
        }
        Command::UpgradeDirectorGradeMax { director_id, grade_type } => {
            let mut notification = None;
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
            if let Some((text, col)) = notification {
                world.add_floating_text(world.boss_pos, text, col, 1.5);
                world.push_audio(crate::audio::AudioEvent::UpgradeGrade);
            }
        }
        Command::PlaceStructure { director_id, pos, .. } => {
            let margin = 45.0;
            let clamped_pos = Vec2::new(
                pos.x.clamp(margin, MAP_WIDTH - margin),
                pos.y.clamp(margin, MAP_HEIGHT - margin),
            );
            let mut can_build = false;
            if can_place_blood_altar(world, clamped_pos) {
                if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                    if director.altar_cooldown <= 0.0
                        && director.spend_points(BLOOD_ALTAR_COST)
                    {
                        director.altar_cooldown = director.altar_max_cooldown;
                        can_build = true;
                    }
                }
            }
            if can_build {
                try_spawn_blood_altar(world, clamped_pos);
                world.push_audio(crate::audio::AudioEvent::AltarBuild);
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
    let restart_prompt = "Нажмите [ R ] для перезапуска сражения";
    let rw = fonts.measure_text(restart_prompt, 26.0).width;
    fonts.draw_text(restart_prompt, (screen_w - rw) / 2.0, screen_h * 0.78, 26.0, GOLD);
}
