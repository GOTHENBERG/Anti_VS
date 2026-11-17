use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub const BLOOD_ALTAR_COST: f32 = 50.0;

/// Минимальная дистанция между центрами алтарей.
pub const ALTAR_MIN_SPACING: f32 = 120.0;

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
/// Возвращает false, если точка невалидна (вне арены или слишком близко к живому алтарю),
/// чтобы Директор не тратил души и кулдаун на заведомо бесполезную постройку.
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
        structure_type: 1,
        pulse_timer: 0.0,
    });

    world.spawn_particles(pos, 25, Color::new(0.6, 0.0, 0.8, 1.0), 90.0);
    world.add_floating_text(pos, "АЛТАРЬ КРОВИ".to_string(), PURPLE, 1.2);
}
