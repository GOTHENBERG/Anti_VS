use macroquad::prelude::*;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::systems::boss_ai::handle_boss_level_up;
use crate::ecs::world::{SoulGemEntity, World};

pub fn update_combat(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let boss_radius = 24.0;
    let is_boss_invulnerable = world.boss_invulnerable_timer > 0.0;

    let has_blade_storm = world.boss_inventory.has_weapon(WeaponType::BladeStorm);
    let has_divine_shield = world.boss_inventory.has_weapon(WeaponType::DivineShield);
    let has_hellfire = world.boss_inventory.has_weapon(WeaponType::Hellfire);

    let mut kamikaze_explosions = Vec::new();

    // 1. Коллизии мобов с Боссом и контактный урон
    let nearby_mobs = world.spatial_grid.query_nearby(boss_pos, 120.0);
    for idx in nearby_mobs {
        if let Some(mob) = world.mobs.get_mut(idx) {
            let dist_sq = mob.pos.distance_squared(boss_pos);
            let collision_dist = mob.radius + boss_radius;

            if dist_sq < collision_dist * collision_dist {
                // Тир 5: Камикадзе взрывается при контакте!
                if mob.tier == 5 {
                    kamikaze_explosions.push((mob.pos, 220.0, 95.0));
                    mob.health.take_damage(9999.0); // самоуничтожение
                } else if !is_boss_invulnerable {
                    let packet = DamagePacket {
                        source_id: Some(mob.id),
                        target_id: 0,
                        amount: mob.damage * dt,
                        damage_type: DamageType::Physical,
                        armor_penetration: 0.0,
                    };
                    let actual_dmg = world.boss_resistances.calculate_damage(&packet, false);
                    world.boss_health.take_damage(actual_dmg);
                    world.boss_inventory.melee_damage_recent += actual_dmg;
                    world.total_damage_dealt_to_boss += actual_dmg;

                    // Скелеты слегка притормаживают босса
                    if mob.tier == 1 {
                        world.boss_vel *= 0.98;
                    }
                }
            }

            // 1.1 Эволюция: Буря Клинков (BladeStorm) — непрерывный контактный вихрь
            if has_blade_storm && dist_sq < 85.0 * 85.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 65.0 * dt,
                    damage_type: DamageType::Physical,
                    armor_penetration: 0.35,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);

                // Отталкивание от вихря
                if !mob.knockback_immune {
                    let push_dir = (mob.pos - boss_pos).normalize_or_zero();
                    mob.knockback_vel += push_dir * 180.0;
                }
            }

            // 1.2 Эволюция: Праведный Щит (DivineShield) — щит с вампиризмом
            if has_divine_shield && dist_sq < 95.0 * 95.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 80.0 * dt,
                    damage_type: DamageType::Holy,
                    armor_penetration: 0.20,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);

                // Вампиризм: 5% урона возвращается в здоровье босса
                world.boss_health.heal(dmg * 0.05);
            }
        }
    }

    // Обработка детонаций Камикадзе
    for (exp_pos, exp_damage, exp_radius) in kamikaze_explosions {
        world.screen_shake = 12.0;
        world.spawn_particles(exp_pos, 45, RED, 220.0);
        world.spawn_particles(exp_pos, 25, ORANGE, 180.0);

        if !is_boss_invulnerable && exp_pos.distance_squared(boss_pos) < exp_radius * exp_radius {
            let packet = DamagePacket {
                source_id: None,
                target_id: 0,
                amount: exp_damage,
                damage_type: DamageType::Fire,
                armor_penetration: 0.25,
            };
            let actual_dmg = world.boss_resistances.calculate_damage(&packet, false);
            world.boss_health.take_damage(actual_dmg);
            world.total_damage_dealt_to_boss += actual_dmg;
            world.add_floating_text(boss_pos, format!("-{:.0}", actual_dmg), RED, 1.4);
        }

        // Взрыв также наносит урон окружающим объектам
        let in_exp = world.spatial_grid.query_nearby(exp_pos, exp_radius);
        for idx in in_exp {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(exp_pos) < exp_radius * exp_radius {
                    mob.health.take_damage(exp_damage * 0.5);
                }
            }
        }
    }

    // 2. Обработка очереди урона (из skills, кислотных луж и т.д.)
    let queue = std::mem::take(&mut world.damage_queue);
    for packet in queue {
        if packet.target_id == 0 {
            if !is_boss_invulnerable {
                let actual_dmg = world.boss_resistances.calculate_damage(&packet, false);
                world.boss_health.take_damage(actual_dmg);
                world.total_damage_dealt_to_boss += actual_dmg;
            }
        } else if let Some(mob) = world.mobs.iter_mut().find(|m| m.id == packet.target_id) {
            let actual_dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
            mob.health.take_damage(actual_dmg);
        }
    }

    // 3. Коллизии снарядов
    let mut boss_hits = Vec::new();
    let mut hit_particles = Vec::new();

    for proj in &mut world.projectiles {
        if proj.lifetime <= 0.0 || proj.pierce == 0 {
            continue;
        }

        if proj.faction == Faction::Boss {
            // Снаряды босса поражают мобов
            let nearby = world.spatial_grid.query_nearby(proj.pos, proj.radius + 30.0);
            for idx in nearby {
                if proj.pierce == 0 {
                    break;
                }
                if let Some(mob) = world.mobs.get_mut(idx) {
                    if mob.health.is_alive() && mob.pos.distance_squared(proj.pos) < (mob.radius + proj.radius).powi(2) {
                        let packet = DamagePacket {
                            source_id: Some(0),
                            target_id: mob.id,
                            amount: proj.damage,
                            damage_type: proj.damage_type,
                            armor_penetration: proj.armor_penetration,
                        };
                        let actual_dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                        mob.health.take_damage(actual_dmg);
                        proj.pierce = proj.pierce.saturating_sub(1);

                        // Отбрасывание моба
                        if !mob.knockback_immune {
                            let push_dir = proj.vel.normalize_or_zero();
                            mob.knockback_vel += push_dir * 120.0;
                        }

                        // Эффекты урона
                        match proj.damage_type {
                            DamageType::Ice => {
                                if mob.tier != 4 {
                                    mob.status.apply_slow(2.5, 0.5);
                                }
                            }
                            DamageType::Fire => {
                                mob.status.apply_burn(3.0, proj.damage * 0.3);
                            }
                            DamageType::Poison => {
                                mob.status.apply_poison(3.5, proj.damage * 0.4);
                            }
                            _ => {}
                        }

                        // Сохраняем частицы для спавна
                        let hit_color = match proj.damage_type {
                            DamageType::Holy => GOLD,
                            DamageType::Ice => SKYBLUE,
                            DamageType::Fire => ORANGE,
                            DamageType::Poison => GREEN,
                            _ => WHITE,
                        };
                        hit_particles.push((mob.pos, hit_color));
                    }
                }
            }
        } else if proj.faction == Faction::Director {
            // Вражеские снаряды (стрелы культистов) поражают Босса
            if proj.pos.distance_squared(boss_pos) < (boss_radius + proj.radius).powi(2) {
                proj.lifetime = 0.0; // снаряд уничтожается
                if !is_boss_invulnerable {
                    let packet = DamagePacket {
                        source_id: Some(proj.id),
                        target_id: 0,
                        amount: proj.damage,
                        damage_type: proj.damage_type,
                        armor_penetration: proj.armor_penetration,
                    };
                    let actual_dmg = world.boss_resistances.calculate_damage(&packet, false);
                    world.boss_health.take_damage(actual_dmg);
                    world.boss_inventory.ranged_damage_recent += actual_dmg;
                    world.total_damage_dealt_to_boss += actual_dmg;
                    boss_hits.push(actual_dmg);
                }
            }
        }
    }

    // Применяем собранные эффекты по боссу и частицы
    for actual_dmg in boss_hits {
        world.add_floating_text(boss_pos, format!("-{:.0}", actual_dmg), RED, 1.1);
        world.spawn_particles(boss_pos, 8, RED, 70.0);
    }
    for (pos, hit_color) in hit_particles {
        world.particles.push(crate::ecs::world::ParticleEntity {
            pos,
            vel: Vec2::new(rand::gen_range(-30.0, 30.0), rand::gen_range(-30.0, 30.0)),
            color: hit_color,
            radius: 3.0,
            lifetime: 0.25,
            max_lifetime: 0.25,
        });
    }

    // Удаляем израсходованные снаряды
    world.projectiles.retain(|p| p.lifetime > 0.0 && p.pierce > 0);

    // 4. Смерть мобов, взрыв трупов (Hellfire) и дроп кристаллов душ
    let mut corpse_explosions = Vec::new();
    let mut dropped_gems = Vec::new();

    world.mobs.retain(|mob| {
        if !mob.health.is_alive() {
            // Эволюция "Адское Пекло": Взрыв трупов горящих врагов!
            if has_hellfire && mob.status.burn_timer > 0.0 {
                corpse_explosions.push((mob.pos, 55.0, 75.0));
            }

            // Шанс дропа особого артефакта "Магнит" (1.5%)
            let is_magnet = rand::gen_range(0.0, 1.0) < 0.015;
            let xp_value = match mob.tier {
                1 => 8.0,
                2 => 20.0,
                3 => 45.0,
                4 => 130.0,
                _ => 160.0,
            };

            dropped_gems.push(SoulGemEntity {
                id: mob.id,
                pos: mob.pos,
                xp_value,
                is_magnet,
            });

            false // удаляем моба
        } else {
            true
        }
    });

    // Обработка взрывов трупов от Hellfire
    for (pos, dmg, radius) in corpse_explosions {
        world.spawn_particles(pos, 18, ORANGE, 140.0);
        let nearby = world.spatial_grid.query_nearby(pos, radius);
        for idx in nearby {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(pos) < radius * radius {
                    mob.health.take_damage(dmg);
                }
            }
        }
    }

    world.gems.extend(dropped_gems);

    // 5. Подбор кристаллов душ Боссом
    let pickup_radius_sq = (boss_radius + 20.0).powi(2);
    let mut collected_xp = 0.0;
    let mut magnet_picked = false;

    world.gems.retain(|gem| {
        let dist_sq = gem.pos.distance_squared(boss_pos);
        if dist_sq < pickup_radius_sq {
            if gem.is_magnet {
                magnet_picked = true;
            }
            collected_xp += gem.xp_value;
            false // кристалл подобран
        } else {
            true
        }
    });

    // Если подобран Магнит — весь оставшийся опыт мгновенно собирается!
    if magnet_picked {
        world.add_floating_text(boss_pos, "МАГНИТ АКТИВИРОВАН!".to_string(), SKYBLUE, 1.5);
        for gem in &world.gems {
            collected_xp += gem.xp_value;
        }
        world.gems.clear();
    }

    if collected_xp > 0.0 {
        let leveled_up = world.boss_inventory.add_xp(collected_xp);
        if leveled_up {
            // Запуск адаптивного Level-Up
            handle_boss_level_up(world);
        }
    }
}
