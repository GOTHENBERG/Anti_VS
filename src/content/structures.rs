use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub const STRUCTURE_ALTAR: u8 = 1;
pub const STRUCTURE_WALL: u8 = 2;
pub const STRUCTURE_OBELISK: u8 = 3;

pub const BLOOD_ALTAR_COST: f32 = 50.0;
pub const ALTAR_MIN_SPACING: f32 = 120.0;

pub const WALL_COST: f32 = 10.0;
pub const WALL_MAX_COOLDOWN: f32 = 2.0;
pub const WALL_LIFETIME: f32 = 60.0;
pub const WALL_RADIUS: f32 = 25.0; // 50x50 px баррикада
pub const OBELISK_RADIUS: f32 = 75.0; // 300x300 px визуальный размер

/// Проверяет, можно ли поставить алтарь в запрошенной точке.
pub fn can_place_blood_altar(world: &World, requested: Vec2) -> bool {
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
    if !can_place_blood_altar(world, requested) {
        return false;
    }

    let margin = 45.0;
    let pos = Vec2::new(
        requested.x.clamp(margin, crate::core::MAP_WIDTH - margin),
        requested.y.clamp(margin, crate::core::MAP_HEIGHT - margin),
    );
    spawn_blood_altar(world, pos);
    true
}

pub fn spawn_blood_altar(world: &mut World, pos: Vec2) {
    let id = world.alloc_id();
    world.structures.push(StructureEntity {
        id,
        pos,
        health: Health::new(650.0),
        resistances: Resistances {
            flat_armor: 12.0,
            physical_res: 0.40,
            fire_res: 0.20,
            ice_res: 0.20,
            poison_res: 1.0, // Невосприимчив к яду
            holy_res: -0.60, // Сильно уязвим к святому урону!
        },
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
    let margin = 35.0;
    if requested.x < margin
        || requested.x > crate::core::MAP_WIDTH - margin
        || requested.y < margin
        || requested.y > crate::core::MAP_HEIGHT - margin
    {
        return false;
    }
    // Нельзя ставить прямо поверх Босса
    if world.boss_pos.distance_squared(requested) < 45.0 * 45.0 {
        return false;
    }
    // Нельзя ставить вплотную к другим живым структурам
    let min_spacing_sq = 44.0 * 44.0;
    !world
        .structures
        .iter()
        .any(|s| s.health.is_alive() && s.pos.distance_squared(requested) < min_spacing_sq)
}

pub fn try_spawn_wall(world: &mut World, requested: Vec2) -> bool {
    if !can_place_wall(world, requested) {
        return false;
    }
    let margin = 35.0;
    let pos = Vec2::new(
        requested.x.clamp(margin, crate::core::MAP_WIDTH - margin),
        requested.y.clamp(margin, crate::core::MAP_HEIGHT - margin),
    );
    spawn_wall(world, pos);
    true
}

pub fn spawn_wall(world: &mut World, pos: Vec2) {
    let id = world.alloc_id();
    world.structures.push(StructureEntity {
        id,
        pos,
        health: Health::new(350.0),
        resistances: Resistances {
            flat_armor: 14.0,
            physical_res: 0.35,
            fire_res: 0.20,
            ice_res: 0.50,
            poison_res: 1.0,
            holy_res: -0.20,
        },
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

    for pos in corners {
        let id = world.alloc_id();
        world.structures.push(StructureEntity {
            id,
            pos,
            health: Health::new(1500.0),
            resistances: Resistances {
                flat_armor: 15.0,
                physical_res: 0.35,
                fire_res: 0.20,
                ice_res: 0.20,
                poison_res: 1.0,
                holy_res: 0.0,
            },
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
