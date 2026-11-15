use crate::ecs::components::combat::{DamagePacket, DamageType};
use crate::ecs::world::{SoulGemEntity, World};

pub fn update_combat(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let boss_radius = 24.0;

    // 1. Коллизии мобов с боссом (мобы наносят урон боссу)
    for mob in &world.mobs {
        if mob.pos.distance_squared(boss_pos) < (mob.radius + boss_radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(mob.id),
                target_id: 0, // 0 = босс
                amount: mob.damage * dt,
                damage_type: DamageType::Physical,
                armor_penetration: 0.0,
            };
            let actual_dmg = world.boss_resistances.calculate_damage(&packet);
            world.boss_health.take_damage(actual_dmg);
        }
    }

    // 2. Коллизии снарядов с мобами
    let mut dead_mobs = Vec::new();
    for proj in &world.projectiles {
        for mob in &mut world.mobs {
            if mob.pos.distance_squared(proj.pos) < (mob.radius + proj.radius).powi(2) {
                let packet = DamagePacket {
                    source_id: Some(0), // от босса
                    target_id: mob.id,
                    amount: proj.damage,
                    damage_type: DamageType::Physical,
                    armor_penetration: 0.1,
                };
                let actual_dmg = mob.resistances.calculate_damage(&packet);
                mob.health.take_damage(actual_dmg);

                if !mob.health.is_alive() {
                    dead_mobs.push((mob.id, mob.pos, mob.tier));
                }
            }
        }
    }

    // 3. Удаление погибших мобов и дроп кристаллов душ
    world.mobs.retain(|m| m.health.is_alive());
    for (id, pos, tier) in dead_mobs {
        world.gems.push(SoulGemEntity {
            id,
            pos,
            xp_value: 10.0 * (tier as f32),
        });
    }

    // 4. Подбор кристаллов душ боссом
    let pickup_radius_sq = (boss_radius + 15.0).powi(2);
    let mut collected_xp = 0.0;
    world.gems.retain(|gem| {
        if gem.pos.distance_squared(boss_pos) < pickup_radius_sq {
            collected_xp += gem.xp_value;
            false // удаляем кристалл
        } else {
            true
        }
    });

    if collected_xp > 0.0 {
        let leveled_up = world.boss_inventory.add_xp(collected_xp);
        if leveled_up {
            // При левелапе можно добавить авто-апгрейд или адаптивный выбор
            auto_level_up(world);
        }
    }
}

fn auto_level_up(world: &mut World) {
    // Простейшая базовая авто-прокачка (для старта)
    if let Some(weapon) = world.boss_inventory.weapons.first_mut() {
        if weapon.level < 8 {
            weapon.level += 1;
            weapon.damage += 8.0;
            weapon.base_cooldown = (weapon.base_cooldown * 0.92).max(0.3);
        }
    }
}
