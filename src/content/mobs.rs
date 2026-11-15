use macroquad::prelude::*;
use crate::ecs::components::combat::{Health, Resistances, StatusEffects};
use crate::ecs::world::{MobEntity, World};

pub struct SquadConfig {
    pub tier: u8,
    pub name: &'static str,
    pub cost: f32,
    pub count: usize,
    pub cooldown: f32,
    pub unlock_spent_threshold: f32,
    pub description: &'static str,
}

pub fn get_squad_config(tier: u8) -> SquadConfig {
    match tier {
        1 => SquadConfig {
            tier: 1,
            name: "Скелеты-пехотинцы",
            cost: 15.0,
            count: 8,
            cooldown: 1.5,
            unlock_spent_threshold: 0.0, // Доступны сразу
            description: "Быстрый спавн пушечного мяса. Замедляют босса при ударе.",
        },
        2 => SquadConfig {
            tier: 2,
            name: "Чёрные гончие",
            cost: 35.0,
            count: 5,
            cooldown: 2.5,
            unlock_spent_threshold: 150.0,
            description: "Скорость 160. Бегут на опережение и отрезают пути отхода.",
        },
        3 => SquadConfig {
            tier: 3,
            name: "Культисты-стрелки",
            cost: 70.0,
            count: 5,
            cooldown: 4.0,
            unlock_spent_threshold: 400.0,
            description: "Дальний бой. Расстреливают босса вне радиуса контактных аур.",
        },
        4 => SquadConfig {
            tier: 4,
            name: "Ледяные големы",
            cost: 150.0,
            count: 2,
            cooldown: 6.0,
            unlock_spent_threshold: 1000.0,
            description: "Тяжелая броня. Не отбрасываются. Иммунитет ко льду, уязвимы к огню.",
        },
        _ => SquadConfig {
            tier: 5,
            name: "Камикадзе",
            cost: 300.0,
            count: 3,
            cooldown: 8.0,
            unlock_spent_threshold: 2500.0,
            description: "Взрываются при контакте, нанося колоссальный урон по площади.",
        },
    }
}

/// Спавн отряда мобов кольцом по всему внешнему периметру экрана
pub fn spawn_squad_perimeter(world: &mut World, tier: u8) {
    let config = get_squad_config(tier);
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Случайный сдвиг угла, чтобы отряды распределялись органично по кольцу
    let base_angle = rand::gen_range(0.0, std::f32::consts::PI * 2.0);

    for i in 0..config.count {
        let id = world.alloc_id();

        // Спавн кольцом на расстоянии 80-120px за видимыми пределами экрана
        let angle = base_angle + (i as f32) * std::f32::consts::PI * 2.0 / (config.count as f32);
        let center = Vec2::new(screen_w * 0.5, screen_h * 0.5);
        let radius_x = screen_w * 0.55 + 80.0;
        let radius_y = screen_h * 0.55 + 80.0;

        let spawn_pos = center + Vec2::new(angle.cos() * radius_x, angle.sin() * radius_y);

        let (hp, speed, dmg, radius, flat_armor, phys_res, ice_res, fire_res, is_undead, knockback_immune) = match tier {
            1 => (32.0, 105.0, 7.0, 10.0, 0.0, 0.05, 0.0, 0.0, true, false), // Скелет
            2 => (50.0, 160.0, 12.0, 11.0, 1.0, 0.10, 0.0, 0.0, false, false), // Чёрная гончая (160 скорость)
            3 => (75.0, 80.0, 16.0, 13.0, 2.0, 0.15, 0.0, 0.0, true, false), // Культист (дальний бой)
            4 => (380.0, 52.0, 32.0, 22.0, 14.0, 0.35, 1.0, -0.50, false, true), // Голем (броня, нет кб, -50% огонь)
            _ => (130.0, 185.0, 0.0, 15.0, 2.0, 0.10, 0.0, 0.0, true, false), // Камикадзе (взрыв при контакте)
        };

        world.mobs.push(MobEntity {
            id,
            pos: spawn_pos,
            vel: Vec2::ZERO,
            speed,
            health: Health::new(hp),
            resistances: Resistances {
                flat_armor,
                physical_res: phys_res,
                fire_res,
                ice_res,
                poison_res: if tier == 4 { 0.5 } else { 0.0 },
                holy_res: if is_undead { -0.25 } else { 0.0 },
            },
            status: StatusEffects::default(),
            damage: dmg,
            tier,
            radius,
            is_undead,
            knockback_immune,
            knockback_vel: Vec2::ZERO,
            attack_timer: rand::gen_range(0.5, 1.5),
        });
    }

    if let Some(director) = world.directors.first_mut() {
        director.total_mobs_spawned += config.count;
    }
}
