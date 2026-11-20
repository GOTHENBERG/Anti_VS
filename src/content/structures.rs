use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub const STRUCTURE_ALTAR: u8 = 1;
pub const STRUCTURE_WALL: u8 = 2;
pub const STRUCTURE_OBELISK: u8 = 3;
pub const STRUCTURE_TIER_ALTAR_BASE: u8 = 20;
pub const MAX_ALTAR_TIER: u8 = 16;

pub const BLOOD_ALTAR_COST: f32 = 50.0;
pub const ALTAR_GRID_SIZE: f32 = 100.0;
pub const ALTAR_HALF_EXTENT: f32 = 50.0; // Хитбокс алтаря 100x100 px
pub const ALTAR_RADIUS: f32 = 50.0;      // Физический радиус для коллизий и урона
pub const ALTAR_MIN_SPACING: f32 = ALTAR_GRID_SIZE;
pub const BUILD_GRID_SIZE: f32 = 50.0;
pub const ALTAR_BASE_COOLDOWN_SECONDS: f32 = 14.0;
pub const ALTAR_COOLDOWN_TIER_STEP: f32 = 7.0;

pub const WALL_COST: f32 = 10.0;
pub const WALL_MAX_COOLDOWN: f32 = 2.0;
pub const WALL_LIFETIME: f32 = 60.0;
pub const WALL_RADIUS: f32 = 25.0; // 50x50 px баррикада
pub const OBELISK_RADIUS: f32 = 75.0; // 300x300 px визуальный размер
pub const OBELISK_FOOTPRINT_HALF_EXTENT: f32 = OBELISK_RADIUS * 2.0;
pub const OBELISK_REWARD_BASE_XP: f32 = 350.0;
pub const OBELISK_REWARD_NEXT_LEVEL_PERCENT: f32 = 0.25; // +25% от шкалы опыта следующего уровня

pub fn structure_type_for_altar_tier(tier: u8) -> u8 {
    let tier = tier.clamp(1, MAX_ALTAR_TIER);
    if tier == 1 { STRUCTURE_ALTAR } else { STRUCTURE_TIER_ALTAR_BASE + tier }
}

pub fn altar_tier_from_structure_type(structure_type: u8) -> Option<u8> {
    if structure_type == STRUCTURE_ALTAR {
        Some(1)
    } else if (STRUCTURE_TIER_ALTAR_BASE + 2..=STRUCTURE_TIER_ALTAR_BASE + MAX_ALTAR_TIER)
        .contains(&structure_type)
    {
        Some(structure_type - STRUCTURE_TIER_ALTAR_BASE)
    } else {
        None
    }
}

pub fn is_altar_structure_type(structure_type: u8) -> bool {
    altar_tier_from_structure_type(structure_type).is_some()
}

pub fn altar_cost(tier: u8) -> f32 {
    let squad_cost = crate::content::mobs::get_squad_config(tier).cost;
    (squad_cost * 2.5).max(BLOOD_ALTAR_COST)
}

pub fn altar_cooldown(tier: u8) -> f32 {
    let tier = tier.clamp(1, MAX_ALTAR_TIER);
    ALTAR_BASE_COOLDOWN_SECONDS + tier.saturating_sub(1) as f32 * ALTAR_COOLDOWN_TIER_STEP
}

pub fn altar_spawn_interval(tier: u8) -> f32 {
    let squad_count = crate::content::mobs::get_squad_config(tier).count.max(1);
    8.0 / squad_count as f32
}

/// Привязывает координату центра алтаря 100x100 к узлам сетки 100 px (2x2 клетки 50 px)
pub fn snap_altar_to_grid(pos: Vec2) -> Vec2 {
    let snap_axis = |value: f32, extent: f32| {
        ((value / ALTAR_GRID_SIZE).floor() * ALTAR_GRID_SIZE + ALTAR_HALF_EXTENT)
            .clamp(ALTAR_HALF_EXTENT + 5.0, extent - ALTAR_HALF_EXTENT - 5.0)
    };
    Vec2::new(snap_axis(pos.x, crate::core::MAP_WIDTH), snap_axis(pos.y, crate::core::MAP_HEIGHT))
}

/// Converts a world coordinate to the center of its 50 px placement cell.
pub fn snap_to_build_grid(pos: Vec2) -> Vec2 {
    let snap_axis = |value: f32, extent: f32| {
        ((value / BUILD_GRID_SIZE).floor() * BUILD_GRID_SIZE + BUILD_GRID_SIZE * 0.5)
            .clamp(BUILD_GRID_SIZE * 0.5, extent - BUILD_GRID_SIZE * 0.5)
    };
    Vec2::new(snap_axis(pos.x, crate::core::MAP_WIDTH), snap_axis(pos.y, crate::core::MAP_HEIGHT))
}

fn apply_carapace(resistances: &mut Resistances, level: u32) {
    resistances.flat_armor += level as f32 * 2.5;
    resistances.physical_res = (resistances.physical_res + level as f32 * 0.05).min(0.70);
}

/// Applies a newly purchased Carapace level to already placed Director structures.
pub fn upgrade_structure_carapace(world: &mut World, levels: u32) {
    if levels == 0 {
        return;
    }
    for structure in &mut world.structures {
        if structure.faction == Faction::Director {
            apply_carapace(&mut structure.resistances, levels);
        }
    }
}

fn distance_sq_to_wall_aabb(point: Vec2, center: Vec2) -> f32 {
    let dx = (point.x - center.x).abs() - WALL_RADIUS;
    let dy = (point.y - center.y).abs() - WALL_RADIUS;
    dx.max(0.0).powi(2) + dy.max(0.0).powi(2)
}

/// Sleeping obelisks are invulnerable to Boss damage until their rift activates.
pub fn boss_can_damage_structure(structure: &StructureEntity) -> bool {
    structure.structure_type != STRUCTURE_OBELISK || structure.is_active
}

/// Проверяет, можно ли поставить алтарь (100x100 px) в запрошенной точке.
/// Алтари можно ставить вплотную к чему угодно (к стенам, другим алтарям и обелискам).
/// Запрещено накладываться на другие алтари, стены и обелиски.
pub fn can_place_blood_altar(world: &World, requested: Vec2) -> bool {
    let requested = snap_altar_to_grid(requested);
    let margin = ALTAR_HALF_EXTENT;
    if requested.x < margin
        || requested.x > crate::core::MAP_WIDTH - margin
        || requested.y < margin
        || requested.y > crate::core::MAP_HEIGHT - margin
    {
        return false;
    }
    !world.structures.iter().any(|s| {
        if !s.health.is_alive() {
            return false;
        }
        let dx = (s.pos.x - requested.x).abs();
        let dy = (s.pos.y - requested.y).abs();
        if is_altar_structure_type(s.structure_type) {
            // Другой алтарь 100x100: расстояние между центрами должно быть не менее 100 по X или Y
            dx < 95.0 && dy < 95.0
        } else if s.structure_type == STRUCTURE_WALL {
            // Стена 50x50: расстояние между центром алтаря (50) и стены (25) не менее 75 по X или Y
            dx < 70.0 && dy < 70.0
        } else if s.structure_type == STRUCTURE_OBELISK {
            // Обелиск 150x150 (half 75): расстояние не менее 75 + 50 = 125 по X или Y
            dx < 120.0 && dy < 120.0
        } else {
            dx < (s.radius + ALTAR_HALF_EXTENT - 5.0) && dy < (s.radius + ALTAR_HALF_EXTENT - 5.0)
        }
    })
}

/// Находит наилучшую клетку для постройки алтаря по алгоритму Maximin Distance от всех живых боссов.
/// Для любого количества боссов (соло или кооп) находит валидную клетку,
/// расстояние от которой до БЛИЖАЙШЕГО живого босса максимально.
pub fn find_furthest_altar_spawn(world: &World) -> Option<Vec2> {
    let mut living_boss_positions = [Vec2::ZERO; 8];
    let mut living_count = 0usize;

    if world.boss_health.is_alive() {
        living_boss_positions[living_count] = world.boss_pos;
        living_count += 1;
    }
    for player in &world.coop_bosses {
        if player.boss_health.is_alive() && living_count < 8 {
            living_boss_positions[living_count] = player.boss_pos;
            living_count += 1;
        }
    }

    if living_count == 0 {
        let center = snap_altar_to_grid(Vec2::new(crate::core::MAP_WIDTH * 0.5, crate::core::MAP_HEIGHT * 0.5));
        return if can_place_blood_altar(world, center) { Some(center) } else { None };
    }

    let living_slice = &living_boss_positions[..living_count];
    let mut best_pos: Option<Vec2> = None;
    let mut max_min_dist_sq = -1.0f32;

    // Быстрый и точный поиск по 25 макро-слотам сетки арены (5x5).
    // Полностью синхронизирован с пространством действий и масками Директора.
    for slot in 0..crate::ai::neural_net::DIRECTOR_ALTAR_HEAD_SIZE.max(25).min(25) {
        let candidate = crate::ai::neural_net::DirectorBrainNet::altar_slot_position(slot);
        if !can_place_blood_altar(world, candidate) {
            continue;
        }

        let mut min_dist_sq = f32::MAX;
        for &boss_pos in living_slice {
            let d_sq = candidate.distance_squared(boss_pos);
            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
            }
        }

        if min_dist_sq > max_min_dist_sq {
            max_min_dist_sq = min_dist_sq;
            best_pos = Some(candidate);
        }
    }

    // Резервный поиск по 4 углам, если все 25 макро-слотов плотно заняты
    if best_pos.is_none() {
        let corners = [
            Vec2::new(70.0, 70.0),
            Vec2::new(crate::core::MAP_WIDTH - 70.0, 70.0),
            Vec2::new(70.0, crate::core::MAP_HEIGHT - 70.0),
            Vec2::new(crate::core::MAP_WIDTH - 70.0, crate::core::MAP_HEIGHT - 70.0),
        ];
        for corner in corners {
            let candidate = snap_altar_to_grid(corner);
            if can_place_blood_altar(world, candidate) {
                let mut min_dist_sq = f32::MAX;
                for &boss_pos in living_slice {
                    let d_sq = candidate.distance_squared(boss_pos);
                    if d_sq < min_dist_sq {
                        min_dist_sq = d_sq;
                    }
                }
                if min_dist_sq > max_min_dist_sq {
                    max_min_dist_sq = min_dist_sq;
                    best_pos = Some(candidate);
                }
            }
        }
    }

    best_pos
}

/// Пытается построить алтарь в запрошенной точке.
pub fn try_spawn_blood_altar(world: &mut World, requested: Vec2) -> bool {
    let requested = snap_altar_to_grid(requested);
    if !can_place_blood_altar(world, requested) {
        return false;
    }

    spawn_blood_altar(world, requested);
    true
}

pub fn spawn_blood_altar(world: &mut World, pos: Vec2) {
    spawn_tier_altar(world, pos, 1);
}

pub fn try_spawn_tier_altar(world: &mut World, requested: Vec2, tier: u8) -> bool {
    let requested = snap_altar_to_grid(requested);
    if !can_place_blood_altar(world, requested) {
        return false;
    }
    spawn_tier_altar(world, requested, tier);
    true
}

pub fn spawn_tier_altar(world: &mut World, pos: Vec2, tier: u8) {
    let pos = snap_altar_to_grid(pos);
    let tier = tier.clamp(1, MAX_ALTAR_TIER);
    let id = world.alloc_id();
    let carapace_level = world.directors.first().map_or(0, |director| director.grade_carapace);
    let mut resistances = Resistances {
        flat_armor: 12.0,
        physical_res: 0.40,
        fire_res: 0.20,
        ice_res: 0.20,
        poison_res: 1.0, // Невосприимчив к яду
        holy_res: -0.60, // Сильно уязвим к святому урону!
    };
    apply_carapace(&mut resistances, carapace_level);
    world.structures.push(StructureEntity {
        id,
        pos,
        health: Health::new(650.0 + (tier.saturating_sub(1) as f32 * 180.0)),
        resistances,
        faction: Faction::Director,
        radius: ALTAR_RADIUS,
        structure_type: structure_type_for_altar_tier(tier),
        pulse_timer: 0.0,
        spawn_timer: 0.0,
        lifetime: -1.0,
        max_lifetime: -1.0,
        is_active: false,
        state_timer: 0.0,
    });

    world.spawn_particles(pos, 35, Color::new(0.6, 0.0, 0.8, 1.0), 120.0);
    world.add_floating_text(pos, format!("АЛТАРЬ ТИРА {}", tier), PURPLE, 1.2);
}

/// Проверяет, можно ли поставить Стену-Баррикаду (50x50 px)
pub fn can_place_wall(world: &World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    // Нельзя ставить прямо поверх Босса
    if distance_sq_to_wall_aabb(world.boss_pos, requested) < 18.0 * 18.0 {
        return false;
    }
    // Adjacent wall cells share an edge exactly; walls cannot overlap other structures.
    !world.structures.iter().any(|structure| {
        if !structure.health.is_alive() {
            return false;
        }
        let dx = (structure.pos.x - requested.x).abs();
        let dy = (structure.pos.y - requested.y).abs();
        if structure.structure_type == STRUCTURE_WALL {
            dx < 45.0 && dy < 45.0
        } else if structure.structure_type == STRUCTURE_OBELISK {
            // Match the visible 300x300 obelisk boundary. Wall cells may touch its
            // edge, but may not overlap the sprite or leave a misleading inner ring.
            let half_extent = OBELISK_FOOTPRINT_HALF_EXTENT + WALL_RADIUS;
            dx < (half_extent - 5.0) && dy < (half_extent - 5.0)
        } else if is_altar_structure_type(structure.structure_type) {
            // Алтарь 100x100: стена 50x50 не может пересекать его хитбокс (half 50 + half 25 = 75)
            dx < 70.0 && dy < 70.0
        } else {
            distance_sq_to_wall_aabb(structure.pos, requested) < structure.radius * structure.radius
        }
    })
}

pub fn try_spawn_wall(world: &mut World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    if !can_place_wall(world, requested) {
        return false;
    }
    spawn_wall(world, requested);
    true
}

pub fn spawn_wall(world: &mut World, pos: Vec2) {
    let pos = snap_to_build_grid(pos);
    let id = world.alloc_id();
    let carapace_level = world.directors.first().map_or(0, |director| director.grade_carapace);
    let mut resistances = Resistances {
        flat_armor: 14.0,
        physical_res: 0.35,
        fire_res: 0.20,
        ice_res: 0.50,
        poison_res: 1.0,
        holy_res: -0.20,
    };
    apply_carapace(&mut resistances, carapace_level);
    world.structures.push(StructureEntity {
        id,
        pos,
        health: Health::new(350.0),
        resistances,
        faction: Faction::Director,
        radius: WALL_RADIUS,
        structure_type: STRUCTURE_WALL,
        pulse_timer: 0.0,
        spawn_timer: 0.0,
        lifetime: WALL_LIFETIME,
        max_lifetime: WALL_LIFETIME,
        is_active: false,
        state_timer: 0.0,
    });

    world.spawn_particles(pos, 15, DARKGRAY, 60.0);
    world.add_floating_text(pos, "БАРРИКАДА".to_string(), LIGHTGRAY, 1.1);
}

/// Спавнит 4 Древних Угловых Обелиска (300x300 px) по углам карты
pub fn spawn_corner_obelisks(world: &mut World) {
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;
    let offset = 260.0;

    let corners = [
        Vec2::new(offset, offset),                 // Северо-Запад
        Vec2::new(map_w - offset, offset),         // Северо-Восток
        Vec2::new(offset, map_h - offset),         // Юго-Запад
        Vec2::new(map_w - offset, map_h - offset), // Юго-Восток
    ];

    for pos in corners.map(snap_to_build_grid) {
        let id = world.alloc_id();
        let carapace_level = world.directors.first().map_or(0, |director| director.grade_carapace);
        let mut resistances = Resistances {
            flat_armor: 15.0,
            physical_res: 0.35,
            fire_res: 0.20,
            ice_res: 0.20,
            poison_res: 1.0,
            holy_res: 0.0,
        };
        apply_carapace(&mut resistances, carapace_level);
        world.structures.push(StructureEntity {
            id,
            pos,
            health: Health::new(1500.0 * world.boss_player_count() as f32),
            resistances,
            faction: Faction::Director,
            radius: OBELISK_RADIUS,
            structure_type: STRUCTURE_OBELISK,
            pulse_timer: 0.0,
            spawn_timer: 0.0,
            lifetime: -1.0,
            max_lifetime: -1.0,
            is_active: false,
            state_timer: 0.0,
        });
    }
}
