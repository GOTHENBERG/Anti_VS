use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub const STRUCTURE_ALTAR: u8 = 1;
pub const STRUCTURE_WALL: u8 = 2;
pub const STRUCTURE_OBELISK: u8 = 3;

pub const BLOOD_ALTAR_COST: f32 = 50.0;
pub const ALTAR_MIN_SPACING: f32 = 120.0;
pub const BUILD_GRID_SIZE: f32 = 50.0;

pub const WALL_COST: f32 = 10.0;
pub const WALL_MAX_COOLDOWN: f32 = 2.0;
pub const WALL_LIFETIME: f32 = 60.0;
pub const WALL_RADIUS: f32 = 25.0; // 50x50 px баррикада
pub const OBELISK_RADIUS: f32 = 75.0; // 300x300 px визуальный размер

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

/// Проверяет, можно ли поставить алтарь в запрошенной точке.
pub fn can_place_blood_altar(world: &World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    let margin = 45.0;
    if requested.x < margin
        || requested.x > crate::core::MAP_WIDTH - margin
        || requested.y < margin
        || requested.y > crate::core::MAP_HEIGHT - margin
    {
        return false;
    }
    let min_spacing_sq = ALTAR_MIN_SPACING * ALTAR_MIN_SPACING;
    !world
        .structures
        .iter()
        .any(|s| s.health.is_alive() && s.pos.distance_squared(requested) < min_spacing_sq)
}

/// Пытается построить алтарь в запрошенной точке.
pub fn try_spawn_blood_altar(world: &mut World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    if !can_place_blood_altar(world, requested) {
        return false;
    }

    spawn_blood_altar(world, requested);
    true
}

pub fn spawn_blood_altar(world: &mut World, pos: Vec2) {
    let pos = snap_to_build_grid(pos);
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
        health: Health::new(650.0),
        resistances,
        faction: Faction::Director,
        radius: 30.0,
        structure_type: STRUCTURE_ALTAR,
        pulse_timer: 0.0,
        spawn_timer: 0.0,
        lifetime: -1.0,
        max_lifetime: -1.0,
        is_active: false,
        state_timer: 0.0,
    });

    world.spawn_particles(pos, 25, Color::new(0.6, 0.0, 0.8, 1.0), 90.0);
    world.add_floating_text(pos, "АЛТАРЬ КРОВИ".to_string(), PURPLE, 1.2);
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
        if structure.structure_type == STRUCTURE_WALL {
            let dx = (structure.pos.x - requested.x).abs();
            let dy = (structure.pos.y - requested.y).abs();
            dx < BUILD_GRID_SIZE && dy < BUILD_GRID_SIZE
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
            health: Health::new(1500.0),
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
