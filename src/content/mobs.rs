use macroquad::prelude::*;
use crate::core::rand;
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
        5 => SquadConfig {
            tier: 5,
            name: "Камикадзе",
            cost: 300.0,
            count: 3,
            cooldown: 8.0,
            unlock_spent_threshold: 2500.0,
            description: "Взрываются при контакте, нанося колоссальный урон по площади.",
        },
        6 => SquadConfig {
            tier: 6,
            name: "Демоны теней",
            cost: 500.0,
            count: 2,
            cooldown: 10.0,
            unlock_spent_threshold: 4000.0,
            description: "Скрытные ассасины. Телепортируются за спину босса, сбивая прицел.",
        },
        7 => SquadConfig {
            tier: 7,
            name: "Огненные элементали",
            cost: 850.0,
            count: 2,
            cooldown: 12.0,
            unlock_spent_threshold: 6500.0,
            description: "100% иммунитет к огню. Плюются огненными шарами и жгут землю.",
        },
        8 => SquadConfig {
            tier: 8,
            name: "Жнец Смерти (Титан)",
            cost: 1800.0,
            count: 1,
            cooldown: 20.0,
            unlock_spent_threshold: 10000.0,
            description: "Мини-босс орды! 2800 HP, гигантская коса смерти, не отбрасывается.",
        },
        9 => SquadConfig {
            tier: 9,
            name: "Дред-Фаланга",
            cost: 250.0,
            count: 8,
            cooldown: 2.5,
            unlock_spent_threshold: 15000.0,
            description: "Элитная нежить в обсидиановых латах. Несгибаемый строй с тяжелыми щитами.",
        },
        10 => SquadConfig {
            tier: 10,
            name: "Адские Гончие",
            cost: 600.0,
            count: 5,
            cooldown: 3.5,
            unlock_spent_threshold: 22000.0,
            description: "Скорость 180! Церберы с огненным дыханием, стремительно окружающие цель.",
        },
        11 => SquadConfig {
            tier: 11,
            name: "Архикультисты Бездны",
            cost: 1100.0,
            count: 5,
            cooldown: 5.5,
            unlock_spent_threshold: 30000.0,
            description: "Снайперы Бездны. Запускают самонаводящиеся темные сгустки издалека.",
        },
        12 => SquadConfig {
            tier: 12,
            name: "Обсидиановые Колоссы",
            cost: 2200.0,
            count: 2,
            cooldown: 8.0,
            unlock_spent_threshold: 40000.0,
            description: "1450 HP, 30 брони. Живые крепости, блокирующие все снаряды босса.",
        },
        13 => SquadConfig {
            tier: 13,
            name: "Проклятые Мучители",
            cost: 3500.0,
            count: 3,
            cooldown: 10.0,
            unlock_spent_threshold: 52000.0,
            description: "Сверхбыстрые взрывники (550 чистый урон). Детонируют при сближении.",
        },
        14 => SquadConfig {
            tier: 14,
            name: "Призрачные Ловчие",
            cost: 5500.0,
            count: 2,
            cooldown: 12.0,
            unlock_spent_threshold: 68000.0,
            description: "Высшие демоны тьмы (1800 HP). Фазовый сдвиг сквозь преграды прямо к боссу.",
        },
        15 => SquadConfig {
            tier: 15,
            name: "Солнечные Архонты",
            cost: 8500.0,
            count: 2,
            cooldown: 15.0,
            unlock_spent_threshold: 85000.0,
            description: "Падшие серафимы (2600 HP, 100% огонь). Испепеляют поле боя солнечным гневом.",
        },
        _ => SquadConfig {
            tier: 16,
            name: "Аватар Забвения (Колосс)",
            cost: 18000.0,
            count: 1,
            cooldown: 25.0,
            unlock_spent_threshold: 110000.0,
            description: "Верховный владыка Бездны! 8500 HP, 45 брони, сокрушительные рассекающие удары.",
        },
    }
}

/// Спавн отряда мобов кольцом вокруг босса на границе видимости
pub fn spawn_squad_perimeter(world: &mut World, tier: u8) {
    spawn_squad_at_dist(world, tier, 820.0);
}

pub fn spawn_squad_at_dist(world: &mut World, tier: u8, ring_radius: f32) {
    // Случайный сдвиг угла, чтобы отряды распределялись органично по кольцу
    let base_angle = rand::gen_range(0.0, std::f32::consts::TAU);
    spawn_squad_arc(world, tier, ring_radius, base_angle, std::f32::consts::TAU);
}

/// Спавн отряда в одном из 8 секторов вокруг Босса (0..7).
/// Даёт Директору управление направлением атаки: окружение, отсечение путей, загон в угол.
pub fn spawn_squad_in_sector(world: &mut World, tier: u8, sector: usize) {
    let sector_idx = sector % 8;
    let center_angle = (sector_idx as f32) * std::f32::consts::TAU / 8.0
        + rand::gen_range(-0.12, 0.12);
    spawn_squad_arc(world, tier, 820.0, center_angle, std::f32::consts::TAU / 8.0);
}

/// Спавн отряда дугой `arc_width` с центром в `center_angle`.
fn spawn_squad_arc(
    world: &mut World,
    tier: u8,
    ring_radius: f32,
    center_angle: f32,
    arc_width: f32,
) {
    let config = get_squad_config(tier);
    let boss_pos = world.boss_pos;
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;

    for i in 0..config.count {
        let id = world.alloc_id();

        // Равномерно распределяем отряд внутри дуги (для полного круга — по всему кольцу)
        let t = (i as f32 + 0.5) / config.count as f32;
        let angle = center_angle + (t - 0.5) * arc_width;
        let offset = Vec2::new(angle.cos() * ring_radius, angle.sin() * ring_radius);
        let raw_spawn_pos = boss_pos + offset;

        let spawn_pos = Vec2::new(
            raw_spawn_pos.x.clamp(20.0, map_w - 20.0),
            raw_spawn_pos.y.clamp(20.0, map_h - 20.0),
        );

        let (base_hp, base_spd, base_dmg, base_rad, mut flat_armor, mut phys_res, ice_res, fire_res, is_undead, knockback_immune) = match tier {
            1 => (32.0, 105.0, 7.0, 10.0, 0.0, 0.05, 0.0, 0.0, true, false), // Скелет
            2 => (50.0, 160.0, 12.0, 11.0, 1.0, 0.10, 0.0, 0.0, false, false), // Чёрная гончая
            3 => (75.0, 80.0, 16.0, 13.0, 2.0, 0.15, 0.0, 0.0, true, false), // Культист
            4 => (380.0, 52.0, 32.0, 22.0, 14.0, 0.35, 1.0, -0.50, false, true), // Голем
            5 => (130.0, 185.0, 0.0, 15.0, 2.0, 0.10, 0.0, 0.0, true, false), // Камикадзе
            6 => (550.0, 140.0, 42.0, 18.0, 8.0, 0.25, 0.20, 0.20, false, false), // Демон теней
            7 => (850.0, 95.0, 55.0, 20.0, 10.0, 0.20, -0.40, 1.00, false, false), // Огненный элементаль
            8 => (2800.0, 68.0, 95.0, 28.0, 24.0, 0.45, 0.30, 0.30, true, true), // Жнец Смерти (Титан)
            9 => (120.0, 115.0, 24.0, 12.0, 6.0, 0.25, 0.10, 0.10, true, true), // Дред-Фаланга
            10 => (190.0, 180.0, 38.0, 14.0, 4.0, 0.20, 0.0, 0.50, false, false), // Адские Гончие
            11 => (320.0, 90.0, 48.0, 15.0, 6.0, 0.25, 0.10, 0.10, true, false), // Архикультисты Бездны
            12 => (1450.0, 58.0, 85.0, 26.0, 30.0, 0.55, 0.80, -0.30, false, true), // Обсидиановые Колоссы
            13 => (650.0, 205.0, 0.0, 18.0, 8.0, 0.20, 0.10, 0.10, true, false), // Проклятые Мучители
            14 => (1800.0, 155.0, 110.0, 22.0, 16.0, 0.40, 0.30, 0.30, false, false), // Призрачные Ловчие
            15 => (2600.0, 110.0, 140.0, 24.0, 22.0, 0.35, -0.20, 1.00, false, false), // Солнечные Архонты
            _ => (8500.0, 75.0, 280.0, 34.0, 45.0, 0.65, 0.50, 0.50, true, true), // Аватар Забвения (Колосс)
        };

        // Применяем улучшения Директора (Mob Grades)
        let (fury_lvl, carapace_lvl) = if let Some(d) = world.directors.first() {
            (d.grade_fury, d.grade_carapace)
        } else {
            (0, 0)
        };

        let mut speed = base_spd * (1.0 + (fury_lvl as f32 * 0.06));
        let mut damage = base_dmg * (1.0 + (fury_lvl as f32 * 0.12));
        flat_armor += carapace_lvl as f32 * 2.5;
        phys_res = (phys_res + (carapace_lvl as f32 * 0.05)).min(0.70);

        // 12% шанс мутации в Элитного Чемпиона (Золотая аура, усиленные статы)
        let is_elite = rand::gen_range(0.0, 1.0) < 0.12;
        let mut hp = base_hp;
        let mut radius = base_rad;
        if is_elite {
            hp *= 1.8;
            damage *= 1.35;
            radius *= 1.25;
            speed *= 1.1;
        }

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
                poison_res: if tier == 4 || tier == 7 { 0.5 } else { 0.0 },
                holy_res: if is_undead { -0.25 } else { 0.0 },
            },
            status: StatusEffects::default(),
            damage,
            tier,
            radius,
            is_undead,
            is_elite,
            knockback_immune,
            knockback_vel: Vec2::ZERO,
            attack_timer: rand::gen_range(0.5, 1.5),
        });
    }

    if let Some(director) = world.directors.first_mut() {
        director.total_mobs_spawned += config.count;
    }
}

/// Точечный спавн одного юнита в указанных координатах (например, скелета от Алтаря Крови)
pub fn spawn_single_mob_at_pos(world: &mut World, tier: u8, pos: Vec2) {
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;
    let clamped_pos = Vec2::new(
        pos.x.clamp(20.0, map_w - 20.0),
        pos.y.clamp(20.0, map_h - 20.0),
    );
    let id = world.alloc_id();
    let (base_hp, base_spd, base_dmg, base_rad, mut flat_armor, mut phys_res, ice_res, fire_res, is_undead, knockback_immune) = match tier {
        1 => (32.0, 105.0, 7.0, 10.0, 0.0, 0.05, 0.0, 0.0, true, false), // Скелет
        2 => (50.0, 160.0, 12.0, 11.0, 1.0, 0.10, 0.0, 0.0, false, false), // Чёрная гончая
        3 => (75.0, 80.0, 16.0, 13.0, 2.0, 0.15, 0.0, 0.0, true, false), // Культист
        _ => (32.0, 105.0, 7.0, 10.0, 0.0, 0.05, 0.0, 0.0, true, false),
    };

    let (fury_lvl, carapace_lvl) = if let Some(d) = world.directors.first() {
        (d.grade_fury, d.grade_carapace)
    } else {
        (0, 0)
    };

    let mut speed = base_spd * (1.0 + (fury_lvl as f32 * 0.06));
    let mut damage = base_dmg * (1.0 + (fury_lvl as f32 * 0.12));
    flat_armor += carapace_lvl as f32 * 2.5;
    phys_res = (phys_res + (carapace_lvl as f32 * 0.05)).min(0.70);

    let is_elite = rand::gen_range(0.0, 1.0) < 0.12;
    let mut hp = base_hp;
    let mut radius = base_rad;
    if is_elite {
        hp *= 1.8;
        damage *= 1.35;
        radius *= 1.25;
        speed *= 1.1;
    }

    world.mobs.push(MobEntity {
        id,
        pos: clamped_pos,
        vel: Vec2::ZERO,
        speed,
        health: Health::new(hp),
        resistances: Resistances {
            flat_armor,
            physical_res: phys_res,
            fire_res,
            ice_res,
            poison_res: if tier == 4 || tier == 7 { 0.5 } else { 0.0 },
            holy_res: if is_undead { -0.25 } else { 0.0 },
        },
        status: StatusEffects::default(),
        damage,
        tier,
        radius,
        is_undead,
        is_elite,
        knockback_immune,
        knockback_vel: Vec2::ZERO,
        attack_timer: rand::gen_range(0.5, 1.5),
    });

    if let Some(director) = world.directors.first_mut() {
        director.total_mobs_spawned += 1;
    }
}
