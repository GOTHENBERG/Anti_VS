#![allow(dead_code)]

use macroquad::prelude::*;

mod content;
mod core;
mod ecs;
mod network;
mod protocol;
mod render;

use crate::content::mobs::{get_squad_config, spawn_squad_perimeter};
use crate::content::structures::{spawn_blood_altar, BLOOD_ALTAR_COST};
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
    let mut world = World::new();
    let mut clock = GameClock::new();
    let mut net_driver = LocalDriver::new();
    let fonts = GameFonts::load();
    let textures = GameTextures::load().await;
    let mut is_paused = false;

    // Камера свободной навигации по большой карте
    let mut cam_pos = world.boss_pos;
    let mut cam_zoom: f32 = 0.95;
    let mut follow_boss = true;
    let auto_screenshot = std::env::args().any(|a| a == "--screenshot");
    let auto_codex = std::env::args().any(|a| a == "--codex");
    let mut show_codex = auto_codex;
    let mut codex_tab: usize = 0;
    let mut frame_count = 0;

    // Начальный разрушаемый алтарь игрока
    spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));

    loop {
        let dt = get_frame_time().min(0.05);
        frame_count += 1;

        if auto_screenshot && frame_count == 10 {
            spawn_squad_perimeter(&mut world, 1);
        }

        // Пауза по P
        if is_key_pressed(KeyCode::P) {
            is_paused = !is_paused;
        }

        // Инфернальный кодекс по I или Esc
        if is_key_pressed(KeyCode::I) {
            show_codex = !show_codex;
        }
        if is_key_pressed(KeyCode::Escape) && show_codex {
            show_codex = false;
        }

        // Скриншот по F12
        if is_key_pressed(KeyCode::F12) {
            get_screen_data().export_png("screenshot.png");
        }

        // Перезапуск игры по R
        let is_game_over = !world.boss_health.is_alive() || clock.is_time_up();
        if is_key_pressed(KeyCode::R) {
            world = World::new();
            clock = GameClock::new();
            cam_pos = world.boss_pos;
            follow_boss = true;
            spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
            is_paused = false;
            show_codex = false;
        }

        // 1. Управление камерой на большой карте: WASD + колесо мыши + центрирование по C / Space
        handle_camera_input(&mut cam_pos, &mut cam_zoom, &mut follow_boss, world.boss_pos, dt);

        let screen_w = screen_width();
        let screen_h = screen_height();
        let world_cam = Camera2D {
            target: cam_pos,
            zoom: Vec2::new(cam_zoom / (screen_w / 2.0), cam_zoom / (screen_h / 2.0)),
            ..Default::default()
        };

        let is_running = !is_game_over && !is_paused && !show_codex;

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
        }

        // 5. Отрисовка мира через камеру
        set_camera(&world_cam);
        render_world(&world, &fonts, &textures);

        // 6. Отрисовка интерфейса на экране (screen-space)
        set_default_camera();
        render_ui(&world, &clock, &fonts, cam_pos, cam_zoom, &textures);

        // 7. Отрисовка экрана паузы, кодекса или завершения игры
        if show_codex {
            draw_codex(&mut codex_tab, &fonts, &textures);
        } else if is_paused && !is_game_over {
            draw_pause_screen(&fonts);
        } else if is_game_over {
            let victory = !world.boss_health.is_alive();
            draw_game_over(&world, &clock, victory, &fonts);
        }

        // Авто-скриншот после отрисовки кадра
        if auto_screenshot && frame_count == 60 {
            // Спавним отряд скелетов на 30-м кадре для наглядности
            get_screen_data().export_png("screenshot.png");
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

    // Центрирование камеры на боссе по клавише C или Space
    if is_key_pressed(KeyCode::C) || is_key_pressed(KeyCode::Space) {
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

fn handle_player_input(world: &World, driver: &mut impl NetworkDriver, world_cam: &Camera2D) {
    if let Some(director) = world.directors.first() {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // 1. Горячие клавиши 1, 2, 3, 4, 5 для спавна отрядов
        let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5];
        for (i, key) in keys.iter().enumerate() {
            if is_key_pressed(*key) {
                try_summon_squad(director, (i + 1) as u8, driver);
            }
        }

        // 2. Клик левой кнопкой мыши по кнопкам на панели призыва
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let btn_width = 185.0;
            let btn_height = 70.0;
            let total_w = 5.0 * btn_width + 4.0 * 12.0;
            let start_x = (screen_w - total_w) / 2.0;
            let btn_y = screen_h - 85.0;

            for i in 0..5 {
                let bx = start_x + (i as f32) * (btn_width + 12.0);
                if mx >= bx && mx <= bx + btn_width && my >= btn_y && my <= btn_y + btn_height {
                    try_summon_squad(director, (i + 1) as u8, driver);
                }
            }
        }

        // 3. Клик правой кнопкой мыши — построить разрушаемый Алтарь Крови в координатах мира!
        if is_mouse_button_pressed(MouseButton::Right) {
            if director.points >= BLOOD_ALTAR_COST {
                let (mx, my) = mouse_position();
                let world_click = world_cam.screen_to_world(Vec2::new(mx, my));
                driver.send_command(Command::PlaceStructure {
                    director_id: director.id,
                    structure_type: 1,
                    pos: world_click.into(),
                });
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
            if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                let config = get_squad_config(tier);
                if director.spend_points(config.cost) {
                    spawn_squad_perimeter(world, tier);
                }
            }
        }
        Command::PlaceStructure { director_id, pos, .. } => {
            if let Some(director) = world.directors.iter_mut().find(|d| d.id == director_id) {
                if director.spend_points(BLOOD_ALTAR_COST) {
                    spawn_blood_altar(world, pos.into());
                }
            }
        }
        _ => {}
    }
}

fn draw_pause_screen(fonts: &GameFonts) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.65));
    let text = "ПАУЗА (НАЖМИТЕ P ДЛЯ ПРОДОЛЖЕНИЯ)";
    let font_size = 36.0;
    let tw = fonts.measure_text(text, font_size).width;
    fonts.draw_text(text, (screen_w - tw) / 2.0, screen_h / 2.0, font_size, YELLOW);
}

fn draw_game_over(world: &World, clock: &GameClock, victory: bool, fonts: &GameFonts) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Затемнение фона
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.85));

    let title = if victory {
        "ПОБЕДА ДИРЕКТОРА ЗЛА! БОСС УНИЧТОЖЕН!"
    } else {
        "ПОРАЖЕНИЕ! БОСС ВЫЖИЛ 30 МИНУТ!"
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
        format!("Время выживания босса: {}", clock.formatted_remaining()),
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
