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

#[inline]
pub fn is_ranged_tier(tier: u8) -> bool {
    matches!(tier, 3 | 7 | 8 | 11 | 15 | 16)
}

impl SquadConfig {
    #[inline]
    pub fn is_ranged(&self) -> bool {
        is_ranged_tier(self.tier)
    }
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
    let target_positions = boss_target_positions(world);
    let target_count = target_positions.len();
    for (target_id, target_pos) in target_positions {
        spawn_squad_arc_at(world, tier, target_id, target_pos, ring_radius, base_angle, std::f32::consts::TAU);
    }
    if let Some(director) = world.directors.first_mut() {
        director.total_mobs_spawned += get_squad_config(tier).count * target_count;
    }
}

/// Спавн отряда в одном из 8 секторов вокруг Босса (0..7).
/// Даёт Директору управление направлением атаки: окружение, отсечение путей, загон в угол.
pub fn spawn_squad_in_sector(world: &mut World, tier: u8, sector: usize) {
    let sector_idx = sector % 8;
    let center_angle = (sector_idx as f32) * std::f32::consts::TAU / 8.0
        + rand::gen_range(-0.12, 0.12);
    let target_positions = boss_target_positions(world);
    let target_count = target_positions.len();
    for (target_id, target_pos) in target_positions {
        spawn_squad_arc_at(world, tier, target_id, target_pos, 820.0, center_angle, std::f32::consts::TAU / 8.0);
    }
    if let Some(director) = world.directors.first_mut() {
        director.total_mobs_spawned += get_squad_config(tier).count * target_count;
    }
}

pub const MIN_BOSS_MOB_SPAWN_DIST: f32 = 420.0;

/// Проверяет и корректирует координаты спавна моба так, чтобы он гарантированно не
/// появился внутри хитбокса или в непосредственной опасной зоне (< 420 px) от ЛЮБОГО живого босса.
pub fn find_safe_mob_spawn_pos(
    world: &World,
    raw_pos: Vec2,
    target_boss_pos: Vec2,
    min_boss_distance: f32,
) -> Vec2 {
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;
    let margin = 35.0;

    let mut pos = Vec2::new(
        raw_pos.x.clamp(margin, map_w - margin),
        raw_pos.y.clamp(margin, map_h - margin),
    );

    let mut live_boss_positions = Vec::with_capacity(world.coop_bosses.len() + 1);
    if world.boss_health.is_alive() {
        live_boss_positions.push(world.boss_pos);
    }
    for player in &world.coop_bosses {
        if player.boss_health.is_alive() {
            live_boss_positions.push(player.boss_pos);
        }
    }

    if live_boss_positions.is_empty() {
        return pos;
    }

    let min_dist_sq = min_boss_distance * min_boss_distance;

    for _ in 0..4 {
        let mut conflict_found = false;
        for &boss_p in &live_boss_positions {
            let diff = pos - boss_p;
            let dist_sq = diff.length_squared();
            if dist_sq < min_dist_sq {
                conflict_found = true;
                let dist = dist_sq.sqrt();
                let push_dir = if dist > 1.0 {
                    diff / dist
                } else {
                    let away = boss_p - target_boss_pos;
                    if away.length_squared() > 1.0 {
                        away.normalize()
                    } else {
                        let center = Vec2::new(map_w * 0.5, map_h * 0.5);
                        let center_dir = center - boss_p;
                        if center_dir.length_squared() > 1.0 {
                            center_dir.normalize()
                        } else {
                            Vec2::new(0.0, -1.0)
                        }
                    }
                };

                let mut candidate = boss_p + push_dir * min_boss_distance;
                let clamped_candidate = Vec2::new(
                    candidate.x.clamp(margin, map_w - margin),
                    candidate.y.clamp(margin, map_h - margin),
                );

                if clamped_candidate.distance_squared(boss_p) < min_dist_sq {
                    let perp = Vec2::new(-push_dir.y, push_dir.x);
                    let alt1 = boss_p + perp * min_boss_distance;
                    let alt2 = boss_p - perp * min_boss_distance;
                    let alt1_c = Vec2::new(alt1.x.clamp(margin, map_w - margin), alt1.y.clamp(margin, map_h - margin));
                    let alt2_c = Vec2::new(alt2.x.clamp(margin, map_w - margin), alt2.y.clamp(margin, map_h - margin));
                    candidate = if alt1_c.distance_squared(boss_p) >= alt2_c.distance_squared(boss_p) {
                        alt1_c
                    } else {
                        alt2_c
                    };
                } else {
                    candidate = clamped_candidate;
                }

                pos = candidate;
            }
        }
        if !conflict_found {
            break;
        }
    }

    pos
}

/// Спавн отряда дугой `arc_width` с центром в `center_angle`.
fn spawn_squad_arc_at(
    world: &mut World,
    tier: u8,
    target_boss_id: u8,
    boss_pos: Vec2,
    ring_radius: f32,
    center_angle: f32,
    arc_width: f32,
) {
    let config = get_squad_config(tier);

    for i in 0..config.count {
        let id = world.alloc_id();

        // Равномерно распределяем отряд внутри дуги (для полного круга — по всему кольцу)
        let t = (i as f32 + 0.5) / config.count as f32;
        let angle = center_angle + (t - 0.5) * arc_width;
        let offset = Vec2::new(angle.cos() * ring_radius, angle.sin() * ring_radius);
        let raw_spawn_pos = boss_pos + offset;

        // Гарантируем, что моб не появится в опасной близости от ЛЮБОГО живого босса
        let spawn_pos = find_safe_mob_spawn_pos(world, raw_spawn_pos, boss_pos, MIN_BOSS_MOB_SPAWN_DIST);

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
            target_boss_id: Some(target_boss_id),
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

}

fn boss_target_positions(world: &World) -> Vec<(u8, Vec2)> {
    let mut positions = Vec::with_capacity(world.coop_bosses.len() + 1);
    if world.boss_health.is_alive() {
        positions.push((1, world.boss_pos));
    }
    positions.extend(
        world
            .coop_bosses
            .iter()
            .filter(|player| player.boss_health.is_alive())
            .map(|player| (player.id, player.boss_pos)),
    );
    positions
}

/// Точечный спавн одного юнита в указанных координатах с возможностью форсировать статус элиты
pub fn spawn_single_mob_at_pos_with_elite(world: &mut World, tier: u8, pos: Vec2, force_elite: bool) {
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

    let (fury_lvl, carapace_lvl) = if let Some(d) = world.directors.first() {
        (d.grade_fury, d.grade_carapace)
    } else {
        (0, 0)
    };

    let mut speed = base_spd * (1.0 + (fury_lvl as f32 * 0.06));
    let mut damage = base_dmg * (1.0 + (fury_lvl as f32 * 0.12));
    flat_armor += carapace_lvl as f32 * 2.5;
    phys_res = (phys_res + (carapace_lvl as f32 * 0.05)).min(0.70);

    let is_elite = force_elite || rand::gen_range(0.0, 1.0) < 0.12;
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
        target_boss_id: None,
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

/// Точечный спавн одного юнита в указанных координатах (например, скелета от Алтаря Крови)
pub fn spawn_single_mob_at_pos(world: &mut World, tier: u8, pos: Vec2) {
    spawn_single_mob_at_pos_with_elite(world, tier, pos, false);
}

/// Спавн элитного стража (гарантированный золотой статус, усиленные HP и урон)
pub fn spawn_elite_guard_at_pos(world: &mut World, tier: u8, pos: Vec2) {
    spawn_single_mob_at_pos_with_elite(world, tier, pos, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_avoids_nearby_bosses() {
        let mut world = World::new();
        // Босс 1 в точке (1000, 1000)
        world.boss_pos = Vec2::new(1000.0, 1000.0);
        world.boss_health.current = 1000.0;

        // Босс 2 в точке (1820, 1000) - ровно на расстоянии радиуса кольца спавна (820 px)!
        world.add_coop_boss(2, Vec2::new(1820.0, 1000.0), "Boss2".to_string(), 0);

        // Сырая точка спавна от Босса 1 строго по оси X в сторону Босса 2: (1000 + 820, 1000) = (1820, 1000)
        let raw_point = Vec2::new(1820.0, 1000.0);
        assert_eq!(raw_point.distance(world.coop_bosses[0].boss_pos), 0.0);

        // Безопасный спавн должен вытолкнуть моба минимум на 420 px от Босса 2
        let safe_pos = find_safe_mob_spawn_pos(&world, raw_point, world.boss_pos, MIN_BOSS_MOB_SPAWN_DIST);
        let dist_to_boss2 = safe_pos.distance(world.coop_bosses[0].boss_pos);
        let dist_to_boss1 = safe_pos.distance(world.boss_pos);

        assert!(dist_to_boss2 >= MIN_BOSS_MOB_SPAWN_DIST - 1.0, "Моб слишком близко к Боссу 2: dist={}", dist_to_boss2);
        assert!(dist_to_boss1 >= MIN_BOSS_MOB_SPAWN_DIST - 1.0, "Моб слишком близко к Боссу 1: dist={}", dist_to_boss1);
    }
}
