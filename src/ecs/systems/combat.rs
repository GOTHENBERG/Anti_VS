use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::{DamageSourceKind, PassiveType, WeaponType};
use crate::ecs::systems::boss_ai::handle_boss_level_up;
use crate::ecs::world::{SoulGemEntity, World};

pub fn update_combat(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let boss_radius = 24.0;
    let is_boss_invulnerable = world.boss_invulnerable_timer > 0.0;

    let has_blade_storm = world.boss_inventory.has_weapon(WeaponType::BladeStorm);
    let has_divine_shield = world.boss_inventory.has_weapon(WeaponType::DivineShield);
    let has_hellfire = world.boss_inventory.has_weapon(WeaponType::Hellfire);
    let has_invincible_bastion = world.boss_inventory.has_weapon(WeaponType::InvincibleBastion);
    let has_octarine_core = world.boss_inventory.has_passive(PassiveType::OctarineCore);
    let has_pyromaniac = world.boss_inventory.has_passive(PassiveType::Pyromaniac);
    let has_immortal_aegis = world.boss_inventory.has_passive(PassiveType::ImmortalAegis);

    let mut kamikaze_explosions = Vec::new();

    // 1. Коллизии мобов с Боссом и контактный урон
    let mut contact_kamikaze_blast = false;
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
                    contact_kamikaze_blast = true;
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
                    world.boss_inventory.record_damage_taken(actual_dmg, packet.damage_type, DamageSourceKind::Melee);
                    world.total_damage_dealt_to_boss += actual_dmg;
                    if actual_dmg > 0.0 {
                        world.boss_damage_taken_timer = 3.0;
                        world.boss_still_time = 0.0;
                    }

                    // Чумные Споры Директора: шанс отравить босса при контактной атаке
                    let pestilence_lvl = world.directors.first().map_or(0, |d| d.grade_pestilence);
                    if pestilence_lvl > 0 {
                        let poison_chance = ((pestilence_lvl as f32) * 0.05).min(0.40);
                        if rand::gen_range(0.0, 1.0) < poison_chance {
                            world.boss_poison_timer = 2.5;
                            world.boss_poison_dps = world.boss_poison_dps.max(6.0 + (pestilence_lvl as f32) * 2.2);
                        }
                    }

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

    if contact_kamikaze_blast {
        world.push_audio(crate::audio::AudioEvent::FireBlast);
    }
    // Звук урона по боссу отключен для чистоты аудиомикса

    // 1.3 Контактный урон аур по разрушаемым Алтарям Крови
    for st in &mut world.structures {
        let dist_sq = st.pos.distance_squared(boss_pos);
        if has_blade_storm && dist_sq < (85.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 75.0 * dt,
                damage_type: DamageType::Physical,
                armor_penetration: 0.35,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
        }

        if has_divine_shield && dist_sq < (95.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 90.0 * dt,
                damage_type: DamageType::Holy,
                armor_penetration: 0.20,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
            world.boss_health.heal(dmg * 0.05);
        }

        if has_hellfire && dist_sq < (95.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 85.0 * dt,
                damage_type: DamageType::Fire,
                armor_penetration: 0.25,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
        }

        if has_invincible_bastion && dist_sq < (105.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 110.0 * dt,
                damage_type: DamageType::Holy,
                armor_penetration: 0.50,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
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
            world.boss_inventory.record_damage_taken(actual_dmg, packet.damage_type, DamageSourceKind::Hazard);
            world.total_damage_dealt_to_boss += actual_dmg;
            if actual_dmg > 0.0 {
                world.boss_damage_taken_timer = 3.0;
                world.boss_still_time = 0.0;
            }
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
    let mut structure_texts = Vec::new();

    // O(1) поиск моба по id вместо O(N) на каждый пакет урона.
    let mob_index: std::collections::HashMap<usize, usize> =
        if queue.iter().any(|p| p.target_id != 0) {
            world
                .mobs
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id, i))
                .collect()
        } else {
            std::collections::HashMap::new()
        };

    for packet in queue {
        if packet.target_id == 0 {
            if !is_boss_invulnerable {
                let actual_dmg = world.boss_resistances.calculate_damage(&packet, false);
                world.boss_health.take_damage(actual_dmg);
                let source_kind = if packet.source_id.is_none() {
                    DamageSourceKind::Hazard
                } else {
                    DamageSourceKind::Melee
                };
                world.boss_inventory.record_damage_taken(actual_dmg, packet.damage_type, source_kind);
                world.total_damage_dealt_to_boss += actual_dmg;
                if actual_dmg > 0.0 {
                    world.boss_damage_taken_timer = 3.0;
                    world.boss_still_time = 0.0;
                }
            }
        } else if let Some(&mob_idx) = mob_index.get(&packet.target_id) {
            if let Some(mob) = world.mobs.get_mut(mob_idx) {
                let actual_dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(actual_dmg);
            }
        } else if let Some(st) = world.structures.iter_mut().find(|s| s.id == packet.target_id) {
            let actual_dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(actual_dmg);
            structure_texts.push((st.pos, format!("-{:.0}", actual_dmg)));
        }
    }
    for (pos, txt) in structure_texts {
        world.add_floating_text(pos, txt, GOLD, 1.0);
    }

    // 3. Коллизии снарядов
    let mut boss_hits = Vec::new();
    let mut hit_particles = Vec::new();
    let mut structure_proj_hits = Vec::new();
    // Переиспользуемый буфер: ноль аллокаций на каждый снаряд.
    let mut nearby_scratch: Vec<usize> = Vec::with_capacity(64);

    for proj in &mut world.projectiles {
        if proj.lifetime <= 0.0 || proj.pierce == 0 {
            continue;
        }

        if proj.faction == Faction::Boss {
            // Снаряды босса поражают мобов
            world
                .spatial_grid
                .query_nearby_into(proj.pos, proj.radius + 30.0, &mut nearby_scratch);
            for &idx in &nearby_scratch {
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

                        // Вампиризм от Кровавой Жатвы (BloodHarvest)
                        if proj.source_weapon == Some(WeaponType::BloodHarvest) {
                            world.boss_health.heal(actual_dmg * 0.20);
                        }

                        // Вампиризм заклинаний от Октаринового Ядра (OctarineCore): 6%
                        if has_octarine_core {
                            world.boss_health.heal(actual_dmg * 0.06);
                        }

                        // Вампиризм огня от Сердца Пироманта (Pyromaniac): 8% от урона огнем
                        if has_pyromaniac && proj.damage_type == DamageType::Fire {
                            world.boss_health.heal(actual_dmg * 0.08);
                        }

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

            // Снаряды босса также атакуют и разрушают Алтари Крови
            for st in &mut world.structures {
                if proj.pierce == 0 {
                    break;
                }
                if st.health.is_alive() && st.pos.distance_squared(proj.pos) < (st.radius + proj.radius).powi(2) {
                    let packet = DamagePacket {
                        source_id: Some(0),
                        target_id: st.id,
                        amount: proj.damage,
                        damage_type: proj.damage_type,
                        armor_penetration: proj.armor_penetration,
                    };
                    let actual_dmg = st.resistances.calculate_damage(&packet, false);
                    st.health.take_damage(actual_dmg);
                    proj.pierce = proj.pierce.saturating_sub(1);
                    hit_particles.push((st.pos, PURPLE));
                    structure_proj_hits.push((st.pos, format!("-{:.0}", actual_dmg)));
                }
            }
        } else if proj.faction == Faction::Director {
            let dist_to_boss_sq = proj.pos.distance_squared(boss_pos);

            // Парирование / уничтожение вражеских снарядов щитом или бурей клинков
            let is_parried = (has_blade_storm && dist_to_boss_sq < 85.0 * 85.0)
                || (has_divine_shield && dist_to_boss_sq < 95.0 * 95.0)
                || (is_boss_invulnerable && dist_to_boss_sq < 70.0 * 70.0);

            if is_parried {
                proj.lifetime = 0.0;
                let spark_color = if has_divine_shield { GOLD } else { SKYBLUE };
                hit_particles.push((proj.pos, spark_color));
                continue;
            }

            // Вражеские снаряды (стрелы культистов) поражают Босса
            if dist_to_boss_sq < (boss_radius + proj.radius).powi(2) {
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
                    world.boss_inventory.record_damage_taken(actual_dmg, packet.damage_type, DamageSourceKind::Ranged);
                    world.total_damage_dealt_to_boss += actual_dmg;
                    boss_hits.push(actual_dmg);
                    if actual_dmg > 0.0 {
                        world.boss_damage_taken_timer = 3.0;
                        world.boss_still_time = 0.0;
                    }

                    // Чумные Споры Директора: шанс отравить босса стрелами культистов
                    let pestilence_lvl = world.directors.first().map_or(0, |d| d.grade_pestilence);
                    if pestilence_lvl > 0 {
                        let poison_chance = ((pestilence_lvl as f32) * 0.08).min(0.50);
                        if rand::gen_range(0.0, 1.0) < poison_chance {
                            world.boss_poison_timer = 2.5;
                            world.boss_poison_dps = world.boss_poison_dps.max(6.0 + (pestilence_lvl as f32) * 2.2);
                        }
                    }
                }
            }
        }
    }

    // Применяем всплывающий урон по структурам
    for (pos, txt) in structure_proj_hits {
        world.add_floating_text(pos, txt, GOLD, 1.1);
    }

    // Применяем собранные эффекты по боссу и частицы
    for actual_dmg in boss_hits {
        world.add_floating_text(boss_pos, format!("-{:.0}", actual_dmg), RED, 1.1);
        world.spawn_particles(boss_pos, 8, RED, 70.0);
    }
    for (pos, hit_color) in hit_particles {
        if world.is_headless {
            break;
        }
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

            // Шанс дропа особого артефакта "Магнит" (1.5%, а с элиты 8%)
            let magnet_chance = if mob.is_elite { 0.08 } else { 0.015 };
            let is_magnet = rand::gen_range(0.0, 1.0) < magnet_chance;

            let mut xp_value = match mob.tier {
                1 => 8.0,
                2 => 20.0,
                3 => 45.0,
                4 => 130.0,
                5 => 160.0,
                6 => 240.0,
                7 => 420.0,
                _ => 1200.0,
            };

            if mob.is_elite {
                xp_value *= 2.5;
            }

            // Высокоуровневые титаны (Тир 8 Жнецы) дропают триплет гигантских сияющих кристаллов!
            if mob.tier == 8 || mob.tier == 16 {
                let chunk = xp_value / 3.0;
                for k in 0..3 {
                    let angle = (k as f32) * 2.0944;
                    let offset = macroquad::prelude::Vec2::new(angle.cos(), angle.sin()) * 26.0;
                    dropped_gems.push(SoulGemEntity {
                        id: mob.id * 10 + k,
                        pos: mob.pos + offset,
                        xp_value: chunk,
                        is_magnet: k == 0 && is_magnet,
                    });
                }
            } else {
                dropped_gems.push(SoulGemEntity {
                    id: mob.id,
                    pos: mob.pos,
                    xp_value,
                    is_magnet,
                });
            }

            false // удаляем моба
        } else {
            true
        }
    });

    // 4.1 Разрушение Алтарей Крови
    let mut destroyed_altars = Vec::new();
    world.structures.retain(|st| {
        if !st.health.is_alive() {
            destroyed_altars.push((st.id, st.pos));
            false
        } else {
            true
        }
    });

    for (st_id, st_pos) in destroyed_altars {
        world.screen_shake = 14.0;
        world.spawn_particles(st_pos, 50, PURPLE, 190.0);
        world.spawn_particles(st_pos, 35, RED, 150.0);
        world.spawn_particles(st_pos, 25, GOLD, 130.0);
        world.add_floating_text(st_pos, "💥 АЛТАРЬ КРОВИ РАЗРУШЕН!".to_string(), GOLD, 1.6);

        // Награда боссу за уничтожение алтаря: много опыта + гарантированный Магнит
        dropped_gems.push(SoulGemEntity {
            id: st_id,
            pos: st_pos,
            xp_value: 150.0,
            is_magnet: true,
        });
    }

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

    // Ограничение и слияние кристаллов душ (Gem Compression) при превышении лимита
    if world.gems.len() > 350 {
        merge_excess_gems(&mut world.gems, 300, boss_pos);
    }

    // 5. Подбор кристаллов душ Боссом
    let pickup_radius_sq = (boss_radius + 20.0).powi(2);
    let mut collected_xp = 0.0;
    let mut magnet_picked = false;
    let mut big_xp_gained = 0.0f32;

    world.gems.retain(|gem| {
        let dist_sq = gem.pos.distance_squared(boss_pos);
        if dist_sq < pickup_radius_sq {
            if gem.is_magnet {
                magnet_picked = true;
            }
            if gem.xp_value >= 120.0 {
                big_xp_gained += gem.xp_value;
            }
            collected_xp += gem.xp_value;
            false // кристалл подобран
        } else {
            true
        }
    });

    if big_xp_gained >= 120.0 {
        world.add_floating_text(boss_pos, format!("+{:.0} XP!", big_xp_gained), GOLD, 1.3);
    }

    // Если подобран Магнит — весь оставшийся опыт мгновенно собирается!
    if magnet_picked {
        world.add_floating_text(boss_pos, "МАГНИТ АКТИВИРОВАН!".to_string(), SKYBLUE, 1.5);
        for gem in &world.gems {
            collected_xp += gem.xp_value;
        }
        world.gems.clear();
    }

    if collected_xp > 0.0 {
        let level_ups = world.boss_inventory.add_xp(collected_xp);
        for _ in 0..level_ups {
            // Сноуболл Босса: увеличение живучести и брони с каждым уровнем
            world.boss_health.max += 40.0;
            world.boss_health.current = (world.boss_health.current + 30.0).min(world.boss_health.max);
            world.boss_resistances.flat_armor += 0.5;

            // Запуск адаптивного Level-Up
            handle_boss_level_up(world);
        }
    }

    // 6. Проверка Эгиды Бессмертия (ImmortalAegis Revive):
    if !world.boss_health.is_alive() && has_immortal_aegis && !world.boss_inventory.has_used_aegis_revive {
        world.boss_inventory.has_used_aegis_revive = true;
        world.boss_health.current = world.boss_health.max;
        world.boss_invulnerable_timer = 3.0; // 3 сек абсолютной неуязвимости
        world.add_floating_text(
            boss_pos,
            "🛡️ ВОСКРЕШЕНИЕ: ЭГИДА БЕССМЕРТИЯ!".to_string(),
            GOLD,
            1.8,
        );
        world.push_audio(crate::audio::AudioEvent::BossLevelUp);
        world.screen_shake = 18.0;
        world.spawn_particles(boss_pos, 60, GOLD, 260.0);
        world.spawn_particles(boss_pos, 40, WHITE, 320.0);

        // Отбрасывание и стан мобов вокруг
        for mob in &mut world.mobs {
            let diff = mob.pos - boss_pos;
            let dist = diff.length();
            if dist < 320.0 && dist > 1.0 {
                mob.vel += diff.normalize() * 480.0;
                mob.health.take_damage(160.0);
            }
        }
    }

    // 7. После удаления погибших мобов индексы в сетке устарели.
    // Перестраиваем её, чтобы сенсоры и скиллы следующего тика видели корректные записи.
    world.rebuild_spatial_grid();
}

/// Слияние избыточных кристаллов душ (Gem Merging):
/// Предотвращает просадки FPS от тысяч объектов на карте при сохранении 100% полученного опыта!
fn merge_excess_gems(gems: &mut Vec<SoulGemEntity>, target_max: usize, boss_pos: Vec2) {
    if gems.len() <= target_max {
        return;
    }

    let boss_dist_min_sq = 150.0 * 150.0;
    let merge_dist: f32 = 70.0;
    let len = gems.len();
    // 1. O(N) слияние близких кристаллов через пространственную сетку 70x70 px.
    // Опыт не теряется: XP поглощённого кристалла переходит представителю ячейки.
    let cell = merge_dist.max(1.0);
    let cols = (crate::core::MAP_WIDTH / cell).ceil() as usize + 1;
    let rows = (crate::core::MAP_HEIGHT / cell).ceil() as usize + 1;
    let mut cell_representative: Vec<Option<usize>> = vec![None; cols * rows];

    for i in 0..len {
        if gems[i].is_magnet || gems[i].xp_value <= 0.0 {
            continue;
        }
        if gems[i].pos.distance_squared(boss_pos) < boss_dist_min_sq {
            continue;
        }

        let cx = (gems[i].pos.x.max(0.0) / cell).floor() as usize;
        let cy = (gems[i].pos.y.max(0.0) / cell).floor() as usize;
        let key = cy.min(rows - 1) * cols + cx.min(cols - 1);

        if let Some(rep) = cell_representative[key] {
            gems[rep].xp_value += gems[i].xp_value;
            gems[i].xp_value = 0.0;
        } else {
            cell_representative[key] = Some(i);
        }
    }

    gems.retain(|g| g.is_magnet || g.xp_value > 0.0);

    // 2. Если кристаллов всё ещё больше целевого капа, укрупняем сетку до 140x140 px
    // Это сохраняет 100% пространственную локальность (сливаются ТОЛЬКО соседние кристаллы, без телепортации!)
    if gems.len() > target_max {
        let cell_large = 140.0f32;
        let cols_l = (crate::core::MAP_WIDTH / cell_large).ceil() as usize + 1;
        let rows_l = (crate::core::MAP_HEIGHT / cell_large).ceil() as usize + 1;
        let mut large_rep: Vec<Option<usize>> = vec![None; cols_l * rows_l];
        let len_l = gems.len();

        for i in 0..len_l {
            if gems[i].is_magnet || gems[i].xp_value <= 0.0 {
                continue;
            }
            if gems[i].pos.distance_squared(boss_pos) < boss_dist_min_sq {
                continue;
            }
            let cx = (gems[i].pos.x.max(0.0) / cell_large).floor() as usize;
            let cy = (gems[i].pos.y.max(0.0) / cell_large).floor() as usize;
            let key = cy.min(rows_l - 1) * cols_l + cx.min(cols_l - 1);

            if let Some(rep) = large_rep[key] {
                gems[rep].xp_value += gems[i].xp_value;
                gems[i].xp_value = 0.0;
            } else {
                large_rep[key] = Some(i);
            }
        }
        gems.retain(|g| g.is_magnet || g.xp_value > 0.0);
    }
}
