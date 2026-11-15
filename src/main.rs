#![allow(dead_code)]

use macroquad::prelude::*;

mod content;
mod core;
mod ecs;
mod network;
mod protocol;
mod render;

use crate::content::mobs::{get_squad_config, spawn_squad_perimeter};
use crate::content::structures::spawn_blood_altar;
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

    // Размещаем тестовый разрушаемый алтарь игрока
    spawn_blood_altar(&mut world, Vec2::new(300.0, 350.0));

    loop {
        let dt = get_frame_time().min(0.05); // Ограничиваем dt при просадках
        clock.tick(dt);

        // 1. Сбор ввода игрока-директора -> генерация Команд (Готово под мультиплеер!)
        handle_player_input(&world, &mut net_driver);

        // 2. Исполнение команд (из локальной очереди или из сети)
        let commands = net_driver.poll_commands();
        for cmd in commands {
            execute_command(&mut world, cmd);
        }

        // 3. Обновление систем (если игра не закончена)
        if world.boss_health.is_alive() && !clock.is_time_up() {
            update_director(&mut world, dt);
            update_boss_ai(&mut world, dt);
            update_meditation(&mut world, dt);
            update_skills(&mut world, dt);
            update_movement(&mut world, dt);
            update_combat(&mut world, dt);
        }

        // 4. Отрисовка мира и UI
        render_world(&world);
        render_ui(&world, &clock);

        // 5. Проверка условий окончания игры
        if !world.boss_health.is_alive() {
            draw_game_over(true);
        } else if clock.is_time_up() {
            draw_game_over(false);
        }

        next_frame().await;
    }
}

fn handle_player_input(world: &World, driver: &mut impl NetworkDriver) {
    if let Some(director) = world.directors.first() {
        let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5];
        for (i, key) in keys.iter().enumerate() {
            if is_key_pressed(*key) {
                let tier = (i + 1) as u8;
                let config = get_squad_config(tier);
                let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
                if is_unlocked && director.squad_cooldowns[i] <= 0.0 && director.points >= config.cost {
                    driver.send_command(Command::SpawnSquad {
                        director_id: director.id,
                        tier,
                    });
                }
            }
        }

        // Клик правой кнопкой мыши — построить разрушаемый Алтарь
        if is_mouse_button_pressed(MouseButton::Right) {
            let mouse_pos = Vec2::new(mouse_position().0, mouse_position().1);
            driver.send_command(Command::PlaceStructure {
                director_id: director.id,
                structure_type: 1,
                pos: mouse_pos.into(),
            });
        }
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
        Command::PlaceStructure { pos, .. } => {
            spawn_blood_altar(world, pos.into());
        }
        _ => {}
    }
}

fn draw_game_over(victory: bool) {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let text = if victory {
        "ПОБЕДА ДИРЕКТОРА! БОСС ПОВЕРЖЕН!"
    } else {
        "ПОРАЖЕНИЕ! БОСС ВЫЖИЛ 30 МИНУТ!"
    };
    let color = if victory { GREEN } else { RED };
    let font_size = 48.0;
    let text_w = measure_text(text, None, font_size as u16, 1.0).width;
    draw_text(text, (screen_w - text_w) / 2.0, screen_h / 2.0, font_size, color);
}
