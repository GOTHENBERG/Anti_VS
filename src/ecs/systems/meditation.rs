use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::components::inventory::PassiveType;
use crate::ecs::world::World;

pub fn update_meditation(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;

    // Снижаем таймер кулдауна после получения урона (3.0 сек)
    if world.boss_damage_taken_timer > 0.0 {
        world.boss_damage_taken_timer = (world.boss_damage_taken_timer - dt).max(0.0);
    }

    // 1. СТАНДАРТНАЯ МЕДИТАЦИЯ
    // Проверяем, есть ли рядом мобы (зона блокировки медитации: 300 px)
    let has_nearby_mobs = world.spatial_grid.query_nearby(boss_pos, 300.0)
        .iter()
        .any(|&idx| {
            if let Some(mob) = world.mobs.get(idx) {
                mob.pos.distance_squared(boss_pos) < 300.0 * 300.0
            } else {
                false
            }
        });

    // Медитация работает ТОЛЬКО если:
    // 1) Босс стоит на месте (скорость < 5 px/sec)
    // 2) Рядом нет врагов (!has_nearby_mobs)
    // 3) Босс НЕ получал урон последние 3 секунды (boss_damage_taken_timer <= 0.0)
    if world.boss_vel.length_squared() < 25.0 && !has_nearby_mobs && world.boss_damage_taken_timer <= 0.0 {
        let prev_still = world.boss_still_time;
        world.boss_still_time += dt;

        // Если стоит более 1.2 секунды подряд -> медитация активирована!
        if world.boss_still_time >= 1.2 {
            if prev_still < 1.2 {
                world.add_floating_text(boss_pos + Vec2::new(0.0, -35.0), "МЕДИТАЦИЯ (+HP, +XP)".to_string(), GREEN, 2.0);
            }
            // Восстановление 2% от максимального здоровья в секунду
            let heal_amount = world.boss_health.max * 0.02 * dt;
            world.boss_health.heal(heal_amount);

            // Получение +30 опыта в секунду
            let xp_gain = 30.0 * dt;
            world.grant_shared_boss_xp(xp_gain);
            crate::ecs::systems::coop::apply_pending_level_ups(world);

            // Визуальные частицы медитации (зеленые и золотые искры вокруг босса)
            if !world.is_headless && rand::gen_range(0.0, 1.0) < 0.40 {
                world.particles.push(crate::ecs::world::ParticleEntity {
                    pos: boss_pos + Vec2::new(rand::gen_range(-25.0, 25.0), rand::gen_range(-25.0, 25.0)),
                    vel: Vec2::new(rand::gen_range(-10.0, 10.0), -40.0),
                    color: Color::new(0.2, 1.0, 0.4, 0.85),
                    radius: 3.0,
                    lifetime: 0.7,
                    max_lifetime: 0.7,
                });
            }
        }
    } else {
        world.boss_still_time = 0.0;
    }

    // 2. РЕГЕНЕРАЦИЯ И КОМАНДНАЯ АУРА ОТ СПОРОВОГО МИЦЕЛИЯ (АСТРАЛЬНЫЕ ГРИБЫ)
    // Работает ВНЕ ЗАВИСИМОСТИ от врагов и урона — главное стоять на месте!
    // В радиусе ауры (растущем с уровнем) исцеляет всех живых союзных боссов!
    let mushroom_lvl = world.boss_inventory.get_passive_level(PassiveType::SporeMushroom);
    if mushroom_lvl > 0 && world.boss_vel.length_squared() < 25.0 {
        let prev_mush = world.boss_mushroom_still_time;
        world.boss_mushroom_still_time += dt;

        // Активация через 0.4 секунды после полной остановки
        if world.boss_mushroom_still_time >= 0.4 {
            let heal_per_sec = 18.0 * (mushroom_lvl as f32) + world.boss_health.max * 0.015 * (mushroom_lvl as f32);
            let heal_amount = heal_per_sec * dt;
            world.boss_health.heal(heal_amount);

            // Командная аура исцеления для союзников в радиусе
            let aura_radius = 160.0 + (mushroom_lvl as f32) * 50.0;
            let aura_r2 = aura_radius * aura_radius;
            for coop in world.coop_bosses.iter_mut() {
                if coop.boss_health.is_alive() && coop.boss_pos.distance_squared(boss_pos) <= aura_r2 {
                    coop.boss_health.heal(heal_amount);
                }
            }

            // Текстовая плашка восстановления раз в секунду
            if prev_mush < 0.4 || (world.boss_mushroom_still_time % 1.0 < dt) {
                world.add_floating_text(
                    boss_pos + Vec2::new(0.0, -48.0),
                    format!("+{:.0} HP (МИЦЕЛИЙ AURA)", heal_per_sec),
                    Color::new(0.1, 0.95, 0.55, 1.0),
                    0.85,
                );
            }

            // Специфичная анимация: биолюминесцентные споры по радиусу ауры
            if !world.is_headless && rand::gen_range(0.0, 1.0) < 0.65 {
                let ang = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                let dist = rand::gen_range(0.0, aura_radius);
                let spore_x = ang.cos() * dist;
                let spore_y = ang.sin() * dist;
                let spore_col = if rand::gen_range(0.0, 1.0) < 0.5 {
                    Color::new(0.15, 1.0, 0.60, 0.90) // изумрудные споры
                } else {
                    Color::new(0.20, 0.90, 1.00, 0.90) // бирюзовые споры
                };
                world.particles.push(crate::ecs::world::ParticleEntity {
                    pos: boss_pos + Vec2::new(spore_x, spore_y),
                    vel: Vec2::new(rand::gen_range(-6.0, 6.0), rand::gen_range(-32.0, -14.0)),
                    color: spore_col,
                    radius: rand::gen_range(2.0, 4.2),
                    lifetime: 0.80,
                    max_lifetime: 0.80,
                });
            }
        }
    } else {
        world.boss_mushroom_still_time = 0.0;
    }

    // Обработка мицелия у кооперативных боссов
    let n_coop = world.coop_bosses.len();
    for i in 0..n_coop {
        let coop_mush_lvl = world.coop_bosses[i].boss_inventory.get_passive_level(PassiveType::SporeMushroom);
        let is_still = world.coop_bosses[i].boss_vel.length_squared() < 25.0;
        let c_pos = world.coop_bosses[i].boss_pos;
        let c_max = world.coop_bosses[i].boss_health.max;

        if coop_mush_lvl > 0 && is_still && world.coop_bosses[i].boss_health.is_alive() {
            let heal_per_sec = 18.0 * (coop_mush_lvl as f32) + c_max * 0.015 * (coop_mush_lvl as f32);
            let heal_amount = heal_per_sec * dt;
            world.coop_bosses[i].boss_health.heal(heal_amount);

            let aura_radius = 160.0 + (coop_mush_lvl as f32) * 50.0;
            let aura_r2 = aura_radius * aura_radius;

            if world.boss_health.is_alive() && world.boss_pos.distance_squared(c_pos) <= aura_r2 {
                world.boss_health.heal(heal_amount);
            }

            for j in 0..n_coop {
                if i != j && world.coop_bosses[j].boss_health.is_alive() && world.coop_bosses[j].boss_pos.distance_squared(c_pos) <= aura_r2 {
                    world.coop_bosses[j].boss_health.heal(heal_amount);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::inventory::{PassiveSlot, PassiveType};

    #[test]
    fn test_spore_mushroom_team_aura_healing() {
        let mut world = World::new();
        world.is_headless = true;
        world.boss_pos = Vec2::new(100.0, 100.0);
        world.boss_vel = Vec2::ZERO;

        // Damage primary boss to 50 HP (max is 500+)
        world.boss_health.current = 50.0;

        // Equip SporeMushroom level 1 (radius = 160 + 50 = 210 px)
        world.boss_inventory.passives.push(PassiveSlot {
            kind: PassiveType::SporeMushroom,
            level: 1,
        });

        // Add Coop Boss 1 inside aura (dist 100 px < 210 px)
        world.add_coop_boss(2, Vec2::new(200.0, 100.0), "NearAlly".to_string(), 0);
        world.coop_bosses[0].boss_health.current = 50.0;

        // Add Coop Boss 2 outside aura (dist 350 px > 210 px)
        world.add_coop_boss(3, Vec2::new(450.0, 100.0), "FarAlly".to_string(), 0);
        world.coop_bosses[1].boss_health.current = 50.0;

        // Stand still for 0.5s to trigger mushroom aura (threshold is 0.4s)
        update_meditation(&mut world, 0.5);

        // Primary boss and nearby ally should be healed
        assert!(world.boss_health.current > 50.0, "Primary boss should be healed by Spore Mushroom");
        assert!(world.coop_bosses[0].boss_health.current > 50.0, "Nearby ally should receive team aura heal");

        // Far ally outside 210px should not be healed
        assert_eq!(world.coop_bosses[1].boss_health.current, 50.0, "Far ally outside radius should not be healed");
    }
}
