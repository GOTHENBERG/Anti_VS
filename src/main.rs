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
    let mut is_paused = false;

    // Начальный разрушаемый алтарь игрока-директора
    spawn_blood_altar(&mut world, Vec2::new(320.0, 360.0));

    loop {
        let dt = get_frame_time().min(0.05);

        // Пауза по пробелу или P
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::P) {
            is_paused = !is_paused;
        }

        // Перезапуск игры по R
        let is_game_over = !world.boss_health.is_alive() || clock.is_time_up();
        if is_key_pressed(KeyCode::R) {
            world = World::new();
            clock = GameClock::new();
            spawn_blood_altar(&mut world, Vec2::new(320.0, 360.0));
            is_paused = false;
        }

        // 1. Сбор ввода игрока-директора -> генерация Команд
        if !is_game_over && !is_paused {
            clock.tick(dt);
            handle_player_input(&world, &mut net_driver);
        }

        // 2. Исполнение сетевых/локальных команд (Command Pattern)
        let commands = net_driver.poll_commands();
        for cmd in commands {
            execute_command(&mut world, cmd);
        }

        // 3. Обновление систем мира
        if !is_game_over && !is_paused {
            update_director(&mut world, dt);
            update_boss_ai(&mut world, dt);
            update_meditation(&mut world, dt);
            update_skills(&mut world, dt);
            update_movement(&mut world, dt);
            update_combat(&mut world, dt);
        }

        // 4. Отрисовка мира и интерфейса
        render_world(&world);
        render_ui(&world, &clock);

        // 5. Отрисовка экрана паузы или завершения игры
        if is_paused && !is_game_over {
            draw_pause_screen();
        } else if is_game_over {
            let victory = !world.boss_health.is_alive();
            draw_game_over(&world, &clock, victory);
        }

        next_frame().await;
    }
}

fn handle_player_input(world: &World, driver: &mut impl NetworkDriver) {
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
            let btn_width = 175.0;
            let btn_height = 68.0;
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

        // 3. Клик правой кнопкой мыши — построить разрушаемый Алтарь Крови
        if is_mouse_button_pressed(MouseButton::Right) {
            if director.points >= BLOOD_ALTAR_COST {
                let (mx, my) = mouse_position();
                driver.send_command(Command::PlaceStructure {
                    director_id: director.id,
                    structure_type: 1,
                    pos: Vec2::new(mx, my).into(),
                });
            }
        }
    }
}

fn try_summon_squad(director: &crate::ecs::world::DirectorState, tier: u8, driver: &mut impl NetworkDriver) {
    let idx = (tier - 1) as usize;
    let config = get_squad_config(tier);
    let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;

    if is_unlocked && director.squad_cooldowns[idx] <= 0.0 && director.points >= config.cost {
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
                    director.squad_cooldowns[(tier - 1) as usize] = config.cooldown;
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

fn draw_pause_screen() {
    let screen_w = screen_width();
    let screen_h = screen_height();

    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.65));
    let text = "ПАУЗА (НАЖМИТЕ ПРОБЕЛ)";
    let font_size = 40.0;
    let tw = measure_text(text, None, font_size as u16, 1.0).width;
    draw_text(text, (screen_w - tw) / 2.0, screen_h / 2.0, font_size, YELLOW);
}

fn draw_game_over(world: &World, clock: &GameClock, victory: bool) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Затемнение фона
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.82));

    let title = if victory {
        "ПОБЕДА ДИРЕКТОРА ЗЛА!"
    } else {
        "ПОРАЖЕНИЕ! БОСС ВЫЖИЛ 30 МИНУТ!"
    };
    let title_color = if victory { GREEN } else { RED };
    let title_font_size = 48.0;
    let tw = measure_text(title, None, title_font_size as u16, 1.0).width;
    draw_text(title, (screen_w - tw) / 2.0, screen_h * 0.28, title_font_size, title_color);

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

    let start_y = screen_h * 0.38;
    for (i, stat) in stats.iter().enumerate() {
        let sw = measure_text(stat, None, 20, 1.0).width;
        draw_text(stat, (screen_w - sw) / 2.0, start_y + (i as f32) * 32.0, 20.0, WHITE);
    }

    // Подсказка перезапуска
    let restart_prompt = "Нажмите [ R ] для перезапуска сражения";
    let rw = measure_text(restart_prompt, None, 26, 1.0).width;
    draw_text(restart_prompt, (screen_w - rw) / 2.0, screen_h * 0.78, 26.0, GOLD);
}
