use rayon::prelude::*;
use crate::ecs::world::World;

pub fn update_movement(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;

    // Параллельное движение мобов к боссу на всех потоках i3-10100F через Rayon!
    world.mobs.par_iter_mut().for_each(|mob| {
        let diff = boss_pos - mob.pos;
        let dist = diff.length();
        if dist > 1.0 {
            let dir = diff / dist;
            mob.pos += dir * (mob.speed * dt);
        }
    });

    // Движение снарядов
    for proj in &mut world.projectiles {
        proj.pos += proj.vel * dt;
        proj.lifetime -= dt;
    }
    world.projectiles.retain(|p| p.lifetime > 0.0);

    // Магнетизм кристаллов душ к боссу
    let magnet_r_sq = world.boss_inventory.magnet_radius * world.boss_inventory.magnet_radius;
    for gem in &mut world.gems {
        let diff = boss_pos - gem.pos;
        let dist_sq = diff.length_squared();
        if dist_sq < magnet_r_sq && dist_sq > 1.0 {
            let dir = diff / dist_sq.sqrt();
            gem.pos += dir * (250.0 * dt);
        }
    }
}
