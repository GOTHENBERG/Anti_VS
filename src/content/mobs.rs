use macroquad::prelude::*;
use crate::ecs::components::combat::{Health, Resistances};
use crate::ecs::world::{MobEntity, World};

pub struct SquadConfig {
    pub cost: f32,
    pub count: usize,
    pub cooldown: f32,
    pub unlock_spent_threshold: f32,
}

pub fn get_squad_config(tier: u8) -> SquadConfig {
    match tier {
        1 => SquadConfig {
            cost: 15.0,
            count: 6,
            cooldown: 1.5,
            unlock_spent_threshold: 0.0, // Доступен сразу
        },
        2 => SquadConfig {
            cost: 35.0,
            count: 4,
            cooldown: 2.5,
            unlock_spent_threshold: 150.0,
        },
        3 => SquadConfig {
            cost: 70.0,
            count: 5,
            cooldown: 4.0,
            unlock_spent_threshold: 400.0,
        },
        4 => SquadConfig {
            cost: 150.0,
            count: 2,
            cooldown: 6.0,
            unlock_spent_threshold: 1000.0,
        },
        _ => SquadConfig {
            cost: 300.0,
            count: 3,
            cooldown: 8.0,
            unlock_spent_threshold: 2500.0,
        },
    }
}

/// Спавн отряда мобов кольцом по периметру экрана
pub fn spawn_squad_perimeter(world: &mut World, tier: u8) {
    let config = get_squad_config(tier);
    let screen_w = screen_width();
    let screen_h = screen_height();

    for i in 0..config.count {
        let id = world.alloc_id();

        // Распределяем спавн по периметру экрана
        let t = (i as f32) / (config.count as f32);
        let perimeter = 2.0 * (screen_w + screen_h);
        let p = t * perimeter;

        let spawn_pos = if p < screen_w {
            Vec2::new(p, -20.0) // Сверху
        } else if p < screen_w + screen_h {
            Vec2::new(screen_w + 20.0, p - screen_w) // Справа
        } else if p < 2.0 * screen_w + screen_h {
            Vec2::new(2.0 * screen_w + screen_h - p, screen_h + 20.0) // Снизу
        } else {
            Vec2::new(-20.0, perimeter - p) // Слева
        };

        let (hp, speed, dmg, radius, flat_armor) = match tier {
            1 => (30.0, 95.0, 6.0, 10.0, 0.0),   // Скелет
            2 => (45.0, 160.0, 10.0, 12.0, 1.0), // Гончая (быстрая)
            3 => (75.0, 75.0, 18.0, 14.0, 3.0),  // Культист
            4 => (260.0, 50.0, 28.0, 20.0, 12.0),// Ледяной голем (бронированный)
            _ => (120.0, 180.0, 65.0, 16.0, 2.0),// Камикадзе
        };

        world.mobs.push(MobEntity {
            id,
            pos: spawn_pos,
            speed,
            health: Health::new(hp),
            resistances: Resistances {
                flat_armor,
                physical_res: (tier as f32 * 0.05).min(0.5),
                fire_res: if tier == 4 { -0.25 } else { 0.0 }, // Голем уязвим к огню!
                ice_res: if tier == 4 { 0.8 } else { 0.0 },   // Голем защищен от холода!
                poison_res: 0.0,
                holy_res: 0.0,
            },
            damage: dmg,
            tier,
            radius,
        });
    }
}
