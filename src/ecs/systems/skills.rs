use macroquad::prelude::Vec2;
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::{ProjectileEntity, World};

pub fn update_skills(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let mut new_projectiles = Vec::new();

    // Находим ближайшего моба для таргетинга
    let nearest_mob_dir = world.mobs.iter()
        .min_by(|a, b| {
            a.pos.distance_squared(boss_pos)
                .partial_cmp(&b.pos.distance_squared(boss_pos))
                .unwrap()
        })
        .map(|m| (m.pos - boss_pos).normalize_or_zero());

    for weapon in &mut world.boss_inventory.weapons {
        weapon.cooldown_timer -= dt;
        if weapon.cooldown_timer <= 0.0 {
            weapon.cooldown_timer = weapon.base_cooldown;

            match weapon.kind {
                WeaponType::WindBlade | WeaponType::BladeStorm => {
                    // Атака во все 4 или 8 сторон
                    let count = if weapon.kind == WeaponType::BladeStorm { 8 } else { 4 };
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * 320.0,
                            damage: weapon.damage,
                            lifetime: 1.5,
                            radius: 8.0,
                        });
                    }
                }
                WeaponType::MagicWand | WeaponType::DeathRay => {
                    // Направленный выстрел в ближайшего моба
                    if let Some(dir) = nearest_mob_dir {
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * 450.0,
                            damage: weapon.damage * 1.5,
                            lifetime: 2.0,
                            radius: 6.0,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    world.projectiles.extend(new_projectiles);
}
