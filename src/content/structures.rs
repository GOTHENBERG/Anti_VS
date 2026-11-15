use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub fn spawn_blood_altar(world: &mut World, pos: Vec2) {
    let id = world.alloc_id();
    world.structures.push(StructureEntity {
        id,
        pos,
        health: Health::new(500.0),
        resistances: Resistances {
            flat_armor: 10.0,
            physical_res: 0.5,
            fire_res: 0.2,
            ice_res: 0.2,
            poison_res: 1.0, // Невосприимчив к яду
            holy_res: -0.5,  // Уязвим к святому урону
        },
        faction: Faction::Director,
        radius: 28.0,
    });
}
