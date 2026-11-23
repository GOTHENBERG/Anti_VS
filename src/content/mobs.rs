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
            description: "Мини-босс орды! 3920 HP, гигантская коса смерти, не отбрасывается.",
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
            description: "2030 HP, 30 брони. Живые крепости, блокирующие все снаряды босса.",
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
            description: "Высшие демоны тьмы (2520 HP). Фазовый сдвиг сквозь преграды прямо к боссу.",
        },
        15 => SquadConfig {
            tier: 15,
            name: "Солнечные Архонты",
            cost: 8500.0,
            count: 2,
            cooldown: 15.0,
            unlock_spent_threshold: 85000.0,
            description: "Падшие серафимы (3640 HP, 100% огонь). Испепеляют поле боя солнечным гневом.",
        },
        16 => SquadConfig {
            tier: 16,
            name: "Аватар Забвения (Колосс)",
            cost: 18000.0,
            count: 1,
            cooldown: 25.0,
            unlock_spent_threshold: 110000.0,
            description: "Верховный владыка Бездны! 11900 HP, 45 брони, сокрушительные рассекающие удары.",
        },
        17 => SquadConfig {
            tier: 17,
            name: "Владыка Пустоты (Герой)",
            cost: 4500.0,
            count: 1,
            cooldown: 60.0,
            unlock_spent_threshold: 0.0,
            description: "Герой Пустоты: Чёрные Дыры со станом, мини-копии, грави-щит. Вдвое медленнее, через 60с входит в Ярость.",
        },
        18 => SquadConfig {
            tier: 18,
            name: "Архидемон Инферно (Герой)",
            cost: 6000.0,
            count: 1,
            cooldown: 60.0,
            unlock_spent_threshold: 0.0,
            description: "Герой Инферно: Лавовые разломы, костяной загон, шипастый панцирь и Судный Метеор. Медленный титан Ярости.",
        },
        _ => SquadConfig {
            tier: 16,
            name: "Аватар Забвения (Колосс)",
            cost: 18000.0,
            count: 1,
            cooldown: 25.0,
            unlock_spent_threshold: 110000.0,
            description: "Верховный владыка Бездны! 11900 HP, 45 брони, сокрушительные рассекающие удары.",
        },
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MobBaseStats {
    pub base_hp: f32,
    pub base_spd: f32,
    pub base_dmg: f32,
    pub base_rad: f32,
    pub flat_armor: f32,
    pub phys_res: f32,
    pub ice_res: f32,
    pub fire_res: f32,
    pub is_undead: bool,
    pub knockback_immune: bool,
}

/// Базовые характеристики мобов и героев.
/// HP всех мобов (тиры 1-16) увеличено на 40%, героев (тиры 17-18) — на 100%.
pub fn get_mob_base_stats(tier: u8) -> MobBaseStats {
    match tier {
        1 => MobBaseStats { base_hp: 44.8, base_spd: 105.0, base_dmg: 7.0, base_rad: 10.0, flat_armor: 0.0, phys_res: 0.05, ice_res: 0.0, fire_res: 0.0, is_undead: true, knockback_immune: false },
        2 => MobBaseStats { base_hp: 70.0, base_spd: 160.0, base_dmg: 12.0, base_rad: 11.0, flat_armor: 1.0, phys_res: 0.10, ice_res: 0.0, fire_res: 0.0, is_undead: false, knockback_immune: false },
        3 => MobBaseStats { base_hp: 105.0, base_spd: 80.0, base_dmg: 16.0, base_rad: 13.0, flat_armor: 2.0, phys_res: 0.15, ice_res: 0.0, fire_res: 0.0, is_undead: true, knockback_immune: false },
        4 => MobBaseStats { base_hp: 532.0, base_spd: 52.0, base_dmg: 32.0, base_rad: 22.0, flat_armor: 14.0, phys_res: 0.35, ice_res: 1.0, fire_res: -0.50, is_undead: false, knockback_immune: true },
        5 => MobBaseStats { base_hp: 182.0, base_spd: 185.0, base_dmg: 0.0, base_rad: 15.0, flat_armor: 2.0, phys_res: 0.10, ice_res: 0.0, fire_res: 0.0, is_undead: true, knockback_immune: false },
        6 => MobBaseStats { base_hp: 770.0, base_spd: 140.0, base_dmg: 42.0, base_rad: 18.0, flat_armor: 8.0, phys_res: 0.25, ice_res: 0.20, fire_res: 0.20, is_undead: false, knockback_immune: false },
        7 => MobBaseStats { base_hp: 1190.0, base_spd: 95.0, base_dmg: 55.0, base_rad: 20.0, flat_armor: 10.0, phys_res: 0.20, ice_res: -0.40, fire_res: 1.00, is_undead: false, knockback_immune: false },
        8 => MobBaseStats { base_hp: 3920.0, base_spd: 68.0, base_dmg: 95.0, base_rad: 28.0, flat_armor: 24.0, phys_res: 0.45, ice_res: 0.30, fire_res: 0.30, is_undead: true, knockback_immune: true },
        9 => MobBaseStats { base_hp: 168.0, base_spd: 115.0, base_dmg: 24.0, base_rad: 12.0, flat_armor: 6.0, phys_res: 0.25, ice_res: 0.10, fire_res: 0.10, is_undead: true, knockback_immune: true },
        10 => MobBaseStats { base_hp: 266.0, base_spd: 180.0, base_dmg: 38.0, base_rad: 14.0, flat_armor: 4.0, phys_res: 0.20, ice_res: 0.0, fire_res: 0.50, is_undead: false, knockback_immune: false },
        11 => MobBaseStats { base_hp: 448.0, base_spd: 90.0, base_dmg: 48.0, base_rad: 15.0, flat_armor: 6.0, phys_res: 0.25, ice_res: 0.10, fire_res: 0.10, is_undead: true, knockback_immune: false },
        12 => MobBaseStats { base_hp: 2030.0, base_spd: 58.0, base_dmg: 85.0, base_rad: 26.0, flat_armor: 30.0, phys_res: 0.55, ice_res: 0.80, fire_res: -0.30, is_undead: false, knockback_immune: true },
        13 => MobBaseStats { base_hp: 910.0, base_spd: 205.0, base_dmg: 0.0, base_rad: 18.0, flat_armor: 8.0, phys_res: 0.20, ice_res: 0.10, fire_res: 0.10, is_undead: true, knockback_immune: false },
        14 => MobBaseStats { base_hp: 2520.0, base_spd: 155.0, base_dmg: 110.0, base_rad: 22.0, flat_armor: 16.0, phys_res: 0.40, ice_res: 0.30, fire_res: 0.30, is_undead: false, knockback_immune: false },
        15 => MobBaseStats { base_hp: 3640.0, base_spd: 110.0, base_dmg: 140.0, base_rad: 24.0, flat_armor: 22.0, phys_res: 0.35, ice_res: -0.20, fire_res: 1.00, is_undead: false, knockback_immune: false },
        16 => MobBaseStats { base_hp: 11900.0, base_spd: 75.0, base_dmg: 280.0, base_rad: 34.0, flat_armor: 45.0, phys_res: 0.65, ice_res: 0.50, fire_res: 0.50, is_undead: true, knockback_immune: true },
        17 => MobBaseStats { base_hp: 32000.0, base_spd: 48.0, base_dmg: 180.0, base_rad: 68.0, flat_armor: 35.0, phys_res: 0.50, ice_res: 0.40, fire_res: 0.40, is_undead: true, knockback_immune: true },
        18 => MobBaseStats { base_hp: 44000.0, base_spd: 45.0, base_dmg: 240.0, base_rad: 78.0, flat_armor: 48.0, phys_res: 0.60, ice_res: 0.20, fire_res: 1.00, is_undead: false, knockback_immune: true },
        _ => MobBaseStats { base_hp: 32000.0, base_spd: 48.0, base_dmg: 180.0, base_rad: 68.0, flat_armor: 35.0, phys_res: 0.50, ice_res: 0.40, fire_res: 0.40, is_undead: true, knockback_immune: true },
    }
}

/// Спавн отряда мобов кольцом вокруг босса на границе видимости
pub fn spawn_squad_perimeter(world: &mut World, tier: u8) {
    spawn_squad_at_dist(world, tier, 820.0);
}

pub fn spawn_squad_at_dist(world: &mut World, tier: u8, ring_radius: f32) {
    let base_angle = rand::gen_range(0.0, std::f32::consts::TAU);
    if tier >= 17 {
        // Спавним строго одного героя вне зависимости от числа боссов
        let target_pos = if world.boss_health.is_alive() {
            world.boss_pos
        } else if let Some(p) = world.coop_bosses.iter().find(|p| p.boss_health.is_alive()) {
            p.boss_pos
        } else {
            world.boss_pos
        };
        spawn_squad_arc_at(world, tier, 1, target_pos, ring_radius, base_angle, std::f32::consts::TAU);
        if let Some(director) = world.directors.first_mut() {
            director.total_mobs_spawned += 1;
        }
        return;
    }

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
    if tier >= 17 {
        let target_pos = if world.boss_health.is_alive() {
            world.boss_pos
        } else if let Some(p) = world.coop_bosses.iter().find(|p| p.boss_health.is_alive()) {
            p.boss_pos
        } else {
            world.boss_pos
        };
        spawn_squad_arc_at(world, tier, 1, target_pos, 820.0, center_angle, std::f32::consts::TAU / 8.0);
        if let Some(director) = world.directors.first_mut() {
            director.total_mobs_spawned += 1;
        }
        return;
    }

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

        let stats = get_mob_base_stats(tier);
        let base_hp = stats.base_hp;
        let base_spd = stats.base_spd;
        let base_dmg = stats.base_dmg;
        let base_rad = stats.base_rad;
        let mut flat_armor = stats.flat_armor;
        let mut phys_res = stats.phys_res;
        let ice_res = stats.ice_res;
        let fire_res = stats.fire_res;
        let is_undead = stats.is_undead;
        let knockback_immune = stats.knockback_immune;

        let is_hero = tier >= 17;
        let living_bosses = world.living_boss_count();
        let coop_hp_mult = 1.0 + (living_bosses.saturating_sub(1) as f32) * 0.33;
        let total_bosses = world.boss_player_count();
        let hero_mult = if is_hero { 1.0 + (total_bosses as f32) * 1.0 } else { 1.0 };
        let mut hp = base_hp * coop_hp_mult * hero_mult;
        let mut radius = base_rad;

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
        let is_elite = !is_hero && rand::gen_range(0.0, 1.0) < 0.12;
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
                poison_res: if tier == 4 || tier == 7 || is_hero { 0.5 } else { 0.0 },
                holy_res: if is_undead { -0.25 } else { 0.0 },
            },
            status: StatusEffects::default(),
            damage,
            tier,
            radius,
            is_undead,
            is_elite,
            knockback_immune: knockback_immune || is_hero,
            knockback_vel: Vec2::ZERO,
            attack_timer: 0.0,
            is_hero,
            hero_enrage_timer: if is_hero { 60.0 } else { 0.0 },
            hero_skill_timers: [0.0; 4],
            attack_count: 0,
            has_duplicated: false,
            owner_hero_id: None,
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
    let stats = get_mob_base_stats(tier);
    let base_hp = stats.base_hp;
    let base_spd = stats.base_spd;
    let base_dmg = stats.base_dmg;
    let base_rad = stats.base_rad;
    let mut flat_armor = stats.flat_armor;
    let mut phys_res = stats.phys_res;
    let ice_res = stats.ice_res;
    let fire_res = stats.fire_res;
    let is_undead = stats.is_undead;
    let knockback_immune = stats.knockback_immune;

    let (fury_lvl, carapace_lvl) = if let Some(d) = world.directors.first() {
        (d.grade_fury, d.grade_carapace)
    } else {
        (0, 0)
    };

    let mut speed = base_spd * (1.0 + (fury_lvl as f32 * 0.06));
    let mut damage = base_dmg * (1.0 + (fury_lvl as f32 * 0.12));
    flat_armor += carapace_lvl as f32 * 2.5;
    phys_res = (phys_res + (carapace_lvl as f32 * 0.05)).min(0.70);

    let is_hero = tier >= 17;
    let is_elite = (force_elite || rand::gen_range(0.0, 1.0) < 0.12) && !is_hero;
    let living_bosses = world.living_boss_count();
    let coop_hp_mult = 1.0 + (living_bosses.saturating_sub(1) as f32) * 0.33;
    let total_bosses = world.boss_player_count();
    let hero_mult = if is_hero { 1.0 + (total_bosses as f32) * 1.0 } else { 1.0 };
    let mut hp = base_hp * coop_hp_mult * hero_mult;
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
            poison_res: if tier == 4 || tier == 7 || is_hero { 0.5 } else { 0.0 },
            holy_res: if is_undead { -0.25 } else { 0.0 },
        },
        status: StatusEffects::default(),
        damage,
        tier,
        radius,
        is_undead,
        is_elite: is_elite && !is_hero,
        knockback_immune: knockback_immune || is_hero,
        knockback_vel: Vec2::ZERO,
        attack_timer: 0.0,
        is_hero,
        hero_enrage_timer: if is_hero { 60.0 } else { 0.0 },
        hero_skill_timers: [0.0; 4],
        attack_count: 0,
        has_duplicated: false,
        owner_hero_id: None,
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
