use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub const BLOOD_ALTAR_COST: f32 = 50.0;

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
