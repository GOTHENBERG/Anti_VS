use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::{AcidPoolEntity, ProjectileEntity, World};

/// Умный тактический выбор цели для направленных атак босса.
/// Оценивает разрушаемые Алтари Крови, скопления опасных культистов-стрелков,
/// камикадзе и непосредственные угрозы вблизи.
pub fn select_tactical_target(world: &World, boss_pos: Vec2, max_range: f32) -> Option<Vec2> {
    let mut best_target = None;
    let mut best_score = -1000.0f32;

    // 1. Оценка живых мобов через Spatial Grid (высший приоритет — непосредственная угроза жизни босса)
    let nearby_indices = world.spatial_grid.query_nearby(boss_pos, max_range);

    for &idx in &nearby_indices {
        if let Some(mob) = world.mobs.get(idx) {
            if !mob.health.is_alive() {
                continue;
            }
            let dist = mob.pos.distance(boss_pos);
            if dist > max_range {
                continue;
            }

            // Базовый вес угрозы по тиру
            let base_threat = match mob.tier {
                // Стрелки-культисты и снайперы
                3 | 11 => 135.0,
                // Камикадзе (смертельно опасны вблизи)
                5 => if dist < 160.0 { 160.0 } else { 75.0 },
                // Боссы и полубоссы (Жнецы, Аватары Бездны)
                8 | 16 => 180.0,
                // Элитная кавалерия / Церберы
                10 => 120.0,
                // Тяжелые фаланги и колоссы
                9 | 12 => 90.0,
                // Мучители и сталкеры
                13 | 14 => 110.0,
                // Гончие (быстрые)
                2 => 80.0,
                // Големы
                4 => 60.0,
                // Скелеты и прочая пехота
                _ => 30.0,
            };

            let mut score = base_threat;

            // Элитные мобы опаснее
            if mob.is_elite {
                score += 80.0;
            }

            // Непосредственная угроза боссу в упор (< 110 px) требует экстренной зачистки!
            if dist < 70.0 {
                score += 150.0;
            } else if dist < 120.0 {
                score += 80.0;
            }

            // Добивание раненых
            if mob.health.current < mob.health.max * 0.35 {
                score += 35.0;
            }

            // Штраф за дальность
            score -= dist * 0.20;

            if score > best_score {
                best_score = score;
                best_target = Some(mob.pos);
            }
        }
    }

    // 2. Структуры не получают отдельного приоритета; активный обелиск имеет тот же вес, что алтарь.
    for st in &world.structures {
        if st.health.is_alive() {
            let dist = st.pos.distance(boss_pos);
            if dist <= max_range {
                let base_score = match st.structure_type {
                    crate::content::structures::STRUCTURE_OBELISK if st.is_active => 110.0,
                    _ if crate::content::structures::is_altar_structure_type(st.structure_type) => 110.0,
                    crate::content::structures::STRUCTURE_WALL => 35.0,
                    _ => -100.0,
                };
                let score = base_score - dist * 0.16;
                if score > best_score {
                    best_score = score;
                    best_target = Some(st.pos);
                }
            }
        }
    }

    best_target
}

/// Находит точку наивысшей концентрации вражеской массы (центр кластера) в пределах радиуса стрельбы.
/// Для AoE-оружий (Метеоры, Сингулярность, Разлом Бездны, Ядовитые споры) целится в центр тяжести
/// наибольшего скопления мобов, максимизируя суммарный сплеш-урон.
pub fn select_cluster_density_target(world: &World, boss_pos: Vec2, max_range: f32, aoe_radius: f32) -> Option<Vec2> {
    let nearby_indices = world.spatial_grid.query_nearby(boss_pos, max_range);
    let aoe_radius_sq = aoe_radius * aoe_radius;
    let mut best_score = 0.0f32;
    let mut best_cluster_center = None;

    // 1. Алтарь Крови оценивается как кандидат кластера
    for st in &world.structures {
        if crate::content::structures::is_altar_structure_type(st.structure_type) && st.health.is_alive() {
            let dist = st.pos.distance(boss_pos);
            if dist <= max_range {
                let cluster_around_altar = world.spatial_grid.query_nearby(st.pos, aoe_radius);
                let mut altar_cluster_threat = 8.0f32;
                let mut weighted_pos = st.pos * 8.0;

                for &m_idx in &cluster_around_altar {
                    if let Some(m) = world.mobs.get(m_idx) {
                        if m.health.is_alive() && m.pos.distance_squared(st.pos) <= aoe_radius_sq {
                            altar_cluster_threat += 1.5;
                            weighted_pos += m.pos * 1.5;
                        }
                    }
                }
                let center = weighted_pos / altar_cluster_threat;
                if altar_cluster_threat > best_score {
                    best_score = altar_cluster_threat;
                    best_cluster_center = Some(center);
                }
            }
        }
    }

    // 2. Сэмплируем кандидатов среди живых мобов
    let candidate_count = nearby_indices.len().min(28);
    for i in 0..candidate_count {
        let idx = nearby_indices[i];
        if let Some(candidate_mob) = world.mobs.get(idx) {
            if !candidate_mob.health.is_alive() {
                continue;
            }
            let center_candidate = candidate_mob.pos;
            if center_candidate.distance_squared(boss_pos) > max_range * max_range {
                continue;
            }

            let cluster_mobs = world.spatial_grid.query_nearby(center_candidate, aoe_radius);
            let mut total_threat = 0.0f32;
            let mut weighted_pos = Vec2::ZERO;

            for &m_idx in &cluster_mobs {
                if let Some(m) = world.mobs.get(m_idx) {
                    if m.health.is_alive() && m.pos.distance_squared(center_candidate) <= aoe_radius_sq {
                        let threat = match m.tier {
                            3 | 11 => 3.5,
                            5 => 4.0,
                            8 | 16 => 6.0,
                            4 | 12 => 3.0,
                            _ => 1.0,
                        } * if m.is_elite { 2.5 } else { 1.0 };

                        let dist_to_boss = m.pos.distance(boss_pos);
                        let proximity_mult = if dist_to_boss < 110.0 { 2.0 } else if dist_to_boss < 180.0 { 1.4 } else { 1.0 };

                        let final_threat = threat * proximity_mult;
                        total_threat += final_threat;
                        weighted_pos += m.pos * final_threat;
                    }
                }
            }

            if total_threat > best_score && total_threat > 0.001 {
                best_score = total_threat;
                best_cluster_center = Some(weighted_pos / total_threat);
            }
        }
    }

    best_cluster_center.or_else(|| select_tactical_target(world, boss_pos, max_range))
}

pub fn update_skills(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let cd_multiplier = world.boss_inventory.get_cooldown_multiplier();
    let armor_pen = world.boss_inventory.get_armor_penetration();
    let dmg_mult = world.boss_inventory.get_damage_multiplier();
    let spd_mult = world.boss_inventory.get_projectile_speed_multiplier();

    // Вращение орбит для Бури Клинков и Праведного Щита
    // Shared match time advances once, even when co-op actors are evaluated through
    // the legacy single-Boss context swap.
    if world.active_boss_id == 1 {
        world.game_time += dt;
    }
    world.blade_storm_angle = (world.blade_storm_angle + dt * 6.0) % (std::f32::consts::PI * 2.0);
    world.divine_shield_angle = (world.divine_shield_angle + dt * 4.0) % (std::f32::consts::PI * 2.0);

    // Таймер активного Луча Смерти
    if let Some((_, _, ref mut timer)) = world.death_ray_active {
        *timer -= dt;
        if *timer <= 0.0 {
            world.death_ray_active = None;
        }
    }

    // Умный выбор тактической цели для направленных атак
    let tactical_target_pos = select_tactical_target(world, boss_pos, 520.0);
    // Кластерное наведение по центру массы толпы для разрушительных AoE атак
    let cluster_target_pos = select_cluster_density_target(world, boss_pos, 550.0, 150.0);

    let mut new_projectiles = Vec::new();
    let mut new_acid_pools = Vec::new();
    let mut death_ray_fire = false;
    let mut death_ray_damage = 95.0 * dmg_mult;

    // Клонируем список оружий для независимой мутации без удержания ссылки на весь мир
    let mut weapons = world.boss_inventory.weapons.clone();

    // 1. Обработка всех оружий в инвентаре босса
    for weapon in &mut weapons {
        weapon.cooldown_timer -= dt;
        if weapon.cooldown_timer <= 0.0 {
            weapon.cooldown_timer = weapon.base_cooldown * cd_multiplier;
            let effective_damage = weapon.damage * dmg_mult;

            // Звуковой эффект применения способности босса
            let sfx = match weapon.kind {
                WeaponType::WindBlade | WeaponType::BladeStorm | WeaponType::ExecutionerAxe |
                WeaponType::WhirlwindCleaver | WeaponType::BloodDaggers | WeaponType::BloodHarvest => {
                    Some(crate::audio::AudioEvent::BladeSlash)
                }
                WeaponType::FireAura | WeaponType::Hellfire | WeaponType::PoisonOrbs | WeaponType::ToxicPestilence => {
                    Some(crate::audio::AudioEvent::FireBlast)
                }
                WeaponType::MagicWand | WeaponType::HolyHammer | WeaponType::DivineShield |
                WeaponType::CelestialBow | WeaponType::StarfallBarrage => {
                    Some(crate::audio::AudioEvent::MagicWand)
                }
                WeaponType::DeathRay | WeaponType::BrimstoneRay | WeaponType::InfernalCataclysm => {
                    Some(crate::audio::AudioEvent::LaserBeam)
                }
                WeaponType::ThunderOrb | WeaponType::ThunderstormWrath | WeaponType::SingularityOrb | WeaponType::EventHorizon => {
                    Some(crate::audio::AudioEvent::LightningStrike)
                }
                WeaponType::IceShards | WeaponType::BlizzardStorm | WeaponType::ChainFrost | WeaponType::AbsoluteZero => {
                    Some(crate::audio::AudioEvent::IceFreeze)
                }
                WeaponType::MissileSwarm | WeaponType::DoomsdayBarrage => {
                    Some(crate::audio::AudioEvent::RocketSwarm)
                }
                WeaponType::ChainLightning | WeaponType::StormGodWrath => {
                    Some(crate::audio::AudioEvent::LightningStrike)
                }
                WeaponType::DivineBell | WeaponType::CathedralAura | WeaponType::AegisOrbiters | WeaponType::InvincibleBastion => {
                    Some(crate::audio::AudioEvent::BladeSlash)
                }
                WeaponType::SoulPhantoms | WeaponType::BansheeWail => {
                    Some(crate::audio::AudioEvent::MagicWand)
                }
                WeaponType::ChaosMeteor | WeaponType::ArmageddonStorm => {
                    Some(crate::audio::AudioEvent::FireBlast)
                }
                WeaponType::VoidRift | WeaponType::SingularityCollapse => {
                    Some(crate::audio::AudioEvent::LaserBeam)
                }
            };
            if let Some(ev) = sfx {
                world.push_audio(ev);
            }

            match weapon.kind {
                // 1.1 Клинок Ветра: веер из 4-6 клинков
                WeaponType::WindBlade => {
                    let count = 4 + (weapon.level / 2) as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (350.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen,
                            lifetime: 1.4,
                            radius: 12.0,
                            pierce: 2,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::WindBlade),
                        });
                    }
                }

                // 1.2 Буря Клинков (ЭВОЛЮЦИЯ): непрерывный круговой вихрь клинков
                WeaponType::BladeStorm => {}

                // 1.3 Огненная Аура: волна огня в радиусе 125px
                WeaponType::FireAura => {
                    let aura_radius = 125.0 + (weapon.level as f32 * 5.0);
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, aura_radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < aura_radius * aura_radius {
                                mob.status.apply_burn(3.0, effective_damage * 0.4);
                                world.damage_queue.push(DamagePacket {
                                    source_id: Some(0),
                                    target_id: mob.id,
                                    amount: effective_damage,
                                    damage_type: DamageType::Fire,
                                    armor_penetration: armor_pen,
                                });
                            }
                        }
                    }

                    // Аура наносит урон и разрушаемым Алтарям Крови
                    for st in &world.structures {
                        if st.pos.distance_squared(boss_pos) < (aura_radius + st.radius).powi(2) {
                            world.damage_queue.push(DamagePacket {
                                source_id: Some(0),
                                target_id: st.id,
                                amount: effective_damage * 0.8,
                                damage_type: DamageType::Fire,
                                armor_penetration: armor_pen,
                            });
                        }
                    }

                    world.spawn_particles(boss_pos, 25, ORANGE, 150.0);
                }

                // 1.4 Адское Пекло (ЭВОЛЮЦИЯ): процентный урон по площади + взрыв трупов
                WeaponType::Hellfire => {
                    let aura_radius = 175.0;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, aura_radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < aura_radius * aura_radius {
                                let percent_dmg = mob.health.max * 0.10 + effective_damage;
                                mob.status.apply_burn(4.0, effective_damage * 0.6);
                                world.damage_queue.push(DamagePacket {
                                    source_id: Some(0),
                                    target_id: mob.id,
                                    amount: percent_dmg,
                                    damage_type: DamageType::Fire,
                                    armor_penetration: armor_pen + 0.25,
                                });
                            }
                        }
                    }

                    for st in &world.structures {
                        if st.pos.distance_squared(boss_pos) < (aura_radius + st.radius).powi(2) {
                            world.damage_queue.push(DamagePacket {
                                source_id: Some(0),
                                target_id: st.id,
                                amount: effective_damage * 1.3,
                                damage_type: DamageType::Fire,
                                armor_penetration: armor_pen + 0.25,
                            });
                        }
                    }

                    world.spawn_particles(boss_pos, 45, RED, 190.0);
                }

                // 1.5 Магический Жезл: быстрые стрелы в тактическую цель
                WeaponType::MagicWand => {
                    if let Some(target_pos) = tactical_target_pos {
                        let dir = (target_pos - boss_pos).normalize_or_zero();
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (480.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 1.0,
                            lifetime: 1.8,
                            radius: 8.0,
                            pierce: 1 + (weapon.level / 3) as u8,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::MagicWand),
                        });
                    }
                }

                // 1.6 Луч Смерти (ЭВОЛЮЦИЯ): прожигающий лазер через весь экран
                WeaponType::DeathRay => {
                    death_ray_fire = true;
                    death_ray_damage = effective_damage;
                }

                // 1.7 Святой Молот: запуск молотов с высоким уроном по нежити и алтарям
                WeaponType::HolyHammer => {
                    let count = 2 + (weapon.level / 3) as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (270.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen,
                            lifetime: 2.2,
                            radius: 14.0,
                            pierce: 5,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::HolyHammer),
                        });
                    }
                }

                // 1.8 Праведный Щит (ЭВОЛЮЦИЯ): кружащийся щит с вампиризмом
                WeaponType::DivineShield => {}

                // 1.9 Ледяные Осколки: веер замедляющих кристаллов
                WeaponType::IceShards => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };

                    let count = 4;
                    for i in 0..count {
                        let offset_angle = ((i as f32) - 1.5) * 0.22;
                        let sin = offset_angle.sin();
                        let cos = offset_angle.cos();
                        let dir = Vec2::new(base_dir.x * cos - base_dir.y * sin, base_dir.x * sin + base_dir.y * cos);

                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (380.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Ice,
                            armor_penetration: armor_pen,
                            lifetime: 1.5,
                            radius: 9.0,
                            pierce: 2,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::IceShards),
                        });
                    }
                }

                // 1.10 Ледниковая Буря (ЭВОЛЮЦИЯ): кольцо заморозки
                WeaponType::BlizzardStorm => {
                    let radius = 220.0;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < radius * radius {
                                if mob.tier != 4 {
                                    mob.status.apply_freeze(2.0);
                                }
                                world.damage_queue.push(DamagePacket {
                                    source_id: Some(0),
                                    target_id: mob.id,
                                    amount: effective_damage,
                                    damage_type: DamageType::Ice,
                                    armor_penetration: armor_pen,
                                });
                            }
                        }
                    }

                    for st in &world.structures {
                        if st.pos.distance_squared(boss_pos) < (radius + st.radius).powi(2) {
                            world.damage_queue.push(DamagePacket {
                                source_id: Some(0),
                                target_id: st.id,
                                amount: effective_damage,
                                damage_type: DamageType::Ice,
                                armor_penetration: armor_pen,
                            });
                        }
                    }

                    world.spawn_particles(boss_pos, 40, SKYBLUE, 160.0);
                }

                // 1.11 Чумные Сферы: кислотные лужи на земле
                WeaponType::PoisonOrbs => {
                    let offset = if let Some(target_pos) = cluster_target_pos {
                        (target_pos - boss_pos).clamp_length_max(190.0)
                    } else {
                        Vec2::new(rand::gen_range(-100.0, 100.0), rand::gen_range(-100.0, 100.0))
                    };
                    new_acid_pools.push(AcidPoolEntity {
                        pos: boss_pos + offset,
                        radius: 70.0,
                        lifetime: 4.5,
                        dps: effective_damage * 0.7,
                        corrosive: false,
                        is_director_source: false,
                    });
                }

                // 1.12 Токсичный Мор (ЭВОЛЮЦИЯ): огромные разъедающие лужи кислоты
                WeaponType::ToxicPestilence => {
                    for _ in 0..2 {
                        let offset = Vec2::new(rand::gen_range(-160.0, 160.0), rand::gen_range(-160.0, 160.0));
                        new_acid_pools.push(AcidPoolEntity {
                            pos: boss_pos + offset,
                            radius: 100.0,
                            lifetime: 6.0,
                            dps: effective_damage,
                            corrosive: true,
                            is_director_source: false,
                        });
                    }
                }

                // 1.13 Топор Палача: тяжелые рассекающие секиры
                WeaponType::ExecutionerAxe => {
                    let count = 2 + (weapon.level / 3) as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (310.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.15,
                            lifetime: 1.8,
                            radius: 14.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::ExecutionerAxe),
                        });
                    }
                }

                // 1.14 Вихревой Секач (ЭВОЛЮЦИЯ): 6 гигантских секачей
                WeaponType::WhirlwindCleaver => {
                    let count = 6;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (380.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.35,
                            lifetime: 2.2,
                            radius: 18.0,
                            pierce: 8,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::WhirlwindCleaver),
                        });
                    }
                }

                // 1.15 Небесный Лук: залпы святых стрел по тактическим целям
                WeaponType::CelestialBow => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };

                    let count = 3 + (weapon.level / 3) as usize;
                    for i in 0..count {
                        let spread = ((i as f32) - (count as f32 - 1.0) / 2.0) * 0.15;
                        let sin = spread.sin();
                        let cos = spread.cos();
                        let dir = Vec2::new(base_dir.x * cos - base_dir.y * sin, base_dir.x * sin + base_dir.y * cos);

                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (540.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen + 0.20,
                            lifetime: 1.8,
                            radius: 10.0,
                            pierce: 3,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::CelestialBow),
                        });
                    }
                }

                // 1.16 Звездопадный Залп (ЭВОЛЮЦИЯ): 8 пронзающих звездных стрел
                WeaponType::StarfallBarrage => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };

                    let count = 8;
                    for i in 0..count {
                        let spread = ((i as f32) - 3.5) * 0.18;
                        let sin = spread.sin();
                        let cos = spread.cos();
                        let dir = Vec2::new(base_dir.x * cos - base_dir.y * sin, base_dir.x * sin + base_dir.y * cos);

                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (620.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen + 0.40,
                            lifetime: 2.2,
                            radius: 13.0,
                            pierce: 6,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::StarfallBarrage),
                        });
                    }
                    world.spawn_particles(boss_pos, 25, GOLD, 180.0);
                }

                // 1.17 Грозовая Сфера: молниевые разряды
                WeaponType::ThunderOrb => {
                    let count = 2 + (weapon.level / 3) as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32) + rand::gen_range(-0.3, 0.3);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 1.0,
                            lifetime: 1.6,
                            radius: 12.0,
                            pierce: 3,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::ThunderOrb),
                        });
                    }
                }

                // 1.18 Гнев Грозы (ЭВОЛЮЦИЯ): каскад молний по арене
                WeaponType::ThunderstormWrath => {
                    let count = 6;
                    for _ in 0..count {
                        let offset = Vec2::new(rand::gen_range(-350.0, 350.0), rand::gen_range(-350.0, 350.0));
                        let strike_pos = boss_pos + offset;
                        world.spawn_particles(strike_pos, 30, YELLOW, 220.0);
                        world.spawn_particles(strike_pos, 15, WHITE, 260.0);

                        let nearby = world.spatial_grid.query_nearby(strike_pos, 120.0);
                        for idx in nearby {
                            if let Some(mob) = world.mobs.get_mut(idx) {
                                if mob.pos.distance_squared(strike_pos) < 120.0 * 120.0 {
                                    mob.status.apply_freeze(1.0); // шоковый стан
                                    world.damage_queue.push(DamagePacket {
                                        source_id: Some(0),
                                        target_id: mob.id,
                                        amount: effective_damage,
                                        damage_type: DamageType::True,
                                        armor_penetration: 1.0,
                                    });
                                }
                            }
                        }
                    }
                }

                // 1.19 Кровавые Кинжалы: быстрые серии клинков
                WeaponType::BloodDaggers => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };

                    let count = 3 + (weapon.level / 3) as usize;
                    for i in 0..count {
                        let spread = ((i as f32) - (count as f32 - 1.0) / 2.0) * 0.12;
                        let sin = spread.sin();
                        let cos = spread.cos();
                        let dir = Vec2::new(base_dir.x * cos - base_dir.y * sin, base_dir.x * sin + base_dir.y * cos);

                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (460.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen,
                            lifetime: 1.2,
                            radius: 8.0,
                            pierce: 1,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::BloodDaggers),
                        });
                    }
                }

                // 1.20 Кровавая Жатва (ЭВОЛЮЦИЯ): веер кинжалов с вампиризмом
                WeaponType::BloodHarvest => {
                    let count = 8 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (520.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.25,
                            lifetime: 1.5,
                            radius: 10.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::BloodHarvest),
                        });
                    }
                }

                // 1.21 Гравитационная Сфера (Черная Дыра): притягивает мобов и взрывается
                WeaponType::SingularityOrb => {
                    let base_dir = if let Some(target_pos) = cluster_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };

                    new_projectiles.push(ProjectileEntity {
                        id: 0,
                        pos: boss_pos,
                        vel: base_dir * (220.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::True,
                        armor_penetration: 0.8,
                        lifetime: 2.2,
                        radius: 18.0,
                        pierce: 10,
                        faction: Faction::Boss,
                        homing_strength: 0.0,
                        source_weapon: Some(WeaponType::SingularityOrb),
                    });
                }

                // 1.22 Горизонт Событий (ЭВОЛЮЦИЯ): червоточина с притяжением и молниями
                WeaponType::EventHorizon => {
                    let count = 2 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (180.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 1.0,
                            lifetime: 3.0,
                            radius: 28.0,
                            pierce: 20,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::EventHorizon),
                        });
                    }
                }

                // 1.23 Цепной Ледяной Шар: отскакивает и замораживает
                WeaponType::ChainFrost => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };
                    let count = 1 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let offset_angle = ((i as f32) - (count as f32 - 1.0) * 0.5) * 0.2;
                        let sin = offset_angle.sin();
                        let cos = offset_angle.cos();
                        let dir = Vec2::new(base_dir.x * cos - base_dir.y * sin, base_dir.x * sin + base_dir.y * cos);

                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Ice,
                            armor_penetration: armor_pen,
                            lifetime: 2.0,
                            radius: 12.0,
                            pierce: 6,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::ChainFrost),
                        });
                    }
                }

                // 1.24 Абсолютный Ноль (ЭВОЛЮЦИЯ): каскад ледяных сфер
                WeaponType::AbsoluteZero => {
                    let count = 4 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (460.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Ice,
                            armor_penetration: armor_pen + 0.3,
                            lifetime: 2.5,
                            radius: 16.0,
                            pierce: 12,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::AbsoluteZero),
                        });
                    }
                }

                // 1.25 Кровавый Гиперлуч: огненный пробивающий луч
                WeaponType::BrimstoneRay => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };
                    new_projectiles.push(ProjectileEntity {
                        id: 0,
                        pos: boss_pos,
                        vel: base_dir * (650.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::Fire,
                        armor_penetration: 0.5,
                        lifetime: 1.5,
                        radius: 20.0,
                        pierce: 15,
                        faction: Faction::Boss,
                        homing_strength: 0.0,
                        source_weapon: Some(WeaponType::BrimstoneRay),
                    });
                }

                // 1.26 Инфернальный Катаклизм (ЭВОЛЮЦИЯ): 4 крестовых плазменных луча
                WeaponType::InfernalCataclysm => {
                    let count = 4;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 0.5;
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (650.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Fire,
                            armor_penetration: 0.75,
                            lifetime: 1.8,
                            radius: 24.0,
                            pierce: 25,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::InfernalCataclysm),
                        });
                    }
                }

                // 1.27 Рой Микроракет: самонаводящиеся ракеты
                WeaponType::MissileSwarm => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };
                    let count = 4 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let spread = ((i as f32) - (count as f32 - 1.0) * 0.5) * 0.35 + rand::gen_range(-0.1, 0.1);
                        let sin = spread.sin();
                        let cos = spread.cos();
                        let dir = Vec2::new(base_dir.x * cos - base_dir.y * sin, base_dir.x * sin + base_dir.y * cos);
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (360.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.25,
                            lifetime: 2.2,
                            radius: 11.0,
                            pierce: 2,
                            faction: Faction::Boss,
                            homing_strength: 7.5,
                            source_weapon: Some(WeaponType::MissileSwarm),
                        });
                    }
                }

                // 1.28 Осада Судного Дня (ЭВОЛЮЦИЯ): ураганный залп кассетных ракет
                WeaponType::DoomsdayBarrage => {
                    let base_dir = if let Some(target_pos) = tactical_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };
                    let count = 8 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let spread = ((i as f32) - (count as f32 - 1.0) * 0.5) * 0.28 + rand::gen_range(-0.12, 0.12);
                        let sin = spread.sin();
                        let cos = spread.cos();
                        let dir = Vec2::new(base_dir.x * cos - base_dir.y * sin, base_dir.x * sin + base_dir.y * cos);
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.50,
                            lifetime: 2.5,
                            radius: 15.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            homing_strength: 9.5,
                            source_weapon: Some(WeaponType::DoomsdayBarrage),
                        });
                    }
                    world.spawn_particles(boss_pos, 25, RED, 180.0);
                }

                // 1.29 Цепная Молния (Static Arc): дуговые разряды молнии
                WeaponType::ChainLightning => {
                    let count = 3 + weapon.bonus_projectiles as usize;
                    for _ in 0..count {
                        let base_dir = if let Some(target_pos) = tactical_target_pos {
                            let jitter: f32 = rand::gen_range(-0.4, 0.4);
                            let raw_dir = (target_pos - boss_pos).normalize_or_zero();
                            Vec2::new(raw_dir.x * jitter.cos() - raw_dir.y * jitter.sin(), raw_dir.x * jitter.sin() + raw_dir.y * jitter.cos())
                        } else {
                            let angle = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                            Vec2::new(angle.cos(), angle.sin())
                        };
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: base_dir * (500.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.50,
                            lifetime: 1.6,
                            radius: 12.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::ChainLightning),
                        });
                    }
                }

                // 1.30 Гнев Бури (ЭВОЛЮЦИЯ): сокрушительный грозовой разряд
                WeaponType::StormGodWrath => {
                    let count = 8 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (580.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.80,
                            lifetime: 2.0,
                            radius: 16.0,
                            pierce: 8,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::StormGodWrath),
                        });
                    }
                    world.spawn_particles(boss_pos, 30, SKYBLUE, 220.0);
                }

                // 1.31 Священный Колокол: резонансный набат
                WeaponType::DivineBell => {
                    let radius = 240.0;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            let d_sq = mob.pos.distance_squared(boss_pos);
                            if d_sq < radius * radius {
                                let push_dir = (mob.pos - boss_pos).normalize_or_zero();
                                if !mob.knockback_immune {
                                    mob.knockback_vel += push_dir * 180.0;
                                }
                                world.damage_queue.push(DamagePacket {
                                    source_id: Some(0),
                                    target_id: mob.id,
                                    amount: effective_damage,
                                    damage_type: DamageType::Holy,
                                    armor_penetration: armor_pen + 0.35,
                                });
                            }
                        }
                    }
                    world.spawn_particles(boss_pos, 25, GOLD, 200.0);
                }

                // 1.32 Аура Собора (ЭВОЛЮЦИЯ): священный хор, очищающий все вокруг
                WeaponType::CathedralAura => {
                    let radius = 340.0;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < radius * radius {
                                world.damage_queue.push(DamagePacket {
                                    source_id: Some(0),
                                    target_id: mob.id,
                                    amount: effective_damage,
                                    damage_type: DamageType::Holy,
                                    armor_penetration: armor_pen + 0.60,
                                });
                            }
                        }
                    }
                    world.spawn_particles(boss_pos, 35, Color::new(1.0, 0.9, 0.3, 0.9), 300.0);
                }

                // 1.33 Голодные Призраки: стая духов, проходящих сквозь стены и врагов
                WeaponType::SoulPhantoms => {
                    let count = 3 + weapon.bonus_projectiles as usize;
                    for _ in 0..count {
                        let angle = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (320.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.40,
                            lifetime: 2.8,
                            radius: 14.0,
                            pierce: 5,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::SoulPhantoms),
                        });
                    }
                }

                // 1.34 Вопль Банши (ЭВОЛЮЦИЯ): 7 призраков Бездны
                WeaponType::BansheeWail => {
                    let count = 7 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (360.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.70,
                            lifetime: 3.5,
                            radius: 18.0,
                            pierce: 12,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::BansheeWail),
                        });
                    }
                    world.spawn_particles(boss_pos, 40, Color::new(0.4, 1.0, 0.7, 0.9), 240.0);
                }

                // 1.35 Орбитальные Бастионы: защитные щиты вокруг босса
                WeaponType::AegisOrbiters => {
                    let count = 2 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let base_angle = (world.game_time * 3.5) + (i as f32 * std::f32::consts::PI * 2.0 / count as f32);
                        let dir = Vec2::new(base_angle.cos(), base_angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos + dir * 65.0,
                            vel: dir * (200.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.3,
                            lifetime: 1.8,
                            radius: 16.0,
                            pierce: 10,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::AegisOrbiters),
                        });
                    }
                }

                // 1.36 Несокрушимая Крепость (ЭВОЛЮЦИЯ): 4 гигантских бастиона
                WeaponType::InvincibleBastion => {
                    let count = 4 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let base_angle = (world.game_time * 4.0) + (i as f32 * std::f32::consts::PI * 0.5);
                        let dir = Vec2::new(base_angle.cos(), base_angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos + dir * 75.0,
                            vel: dir * (250.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen + 0.6,
                            lifetime: 2.2,
                            radius: 22.0,
                            pierce: 25,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::InvincibleBastion),
                        });
                    }
                }

                // 1.37 Хаотический Метеор: падающий метеорит с колоссальным взрывом
                WeaponType::ChaosMeteor => {
                    let target = cluster_target_pos.unwrap_or(boss_pos + Vec2::new(rand::gen_range(-150.0, 150.0), rand::gen_range(-150.0, 150.0)));
                    let dir = (target - boss_pos).normalize_or_zero();
                    new_projectiles.push(ProjectileEntity {
                        id: 0,
                        pos: boss_pos,
                        vel: dir * (400.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::Fire,
                        armor_penetration: 0.50,
                        lifetime: 1.5,
                        radius: 24.0,
                        pierce: 1,
                        faction: Faction::Boss,
                        homing_strength: 0.0,
                        source_weapon: Some(WeaponType::ChaosMeteor),
                    });
                }

                // 1.38 Шторм Армагеддона (ЭВОЛЮЦИЯ): 5 метеоритов
                WeaponType::ArmageddonStorm => {
                    let count = 5 + weapon.bonus_projectiles as usize;
                    for _ in 0..count {
                        let offset = Vec2::new(rand::gen_range(-300.0, 300.0), rand::gen_range(-300.0, 300.0));
                        let target = boss_pos + offset;
                        let dir = (target - boss_pos).normalize_or_zero();
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (450.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Fire,
                            armor_penetration: 0.75,
                            lifetime: 1.6,
                            radius: 28.0,
                            pierce: 1,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::ArmageddonStorm),
                        });
                    }
                    world.spawn_particles(boss_pos, 40, ORANGE, 260.0);
                }

                // 1.39 Разрыв Бездны: космическая воронка, притягивающая врагов
                WeaponType::VoidRift => {
                    let base_dir = if let Some(target_pos) = cluster_target_pos {
                        (target_pos - boss_pos).normalize_or_zero()
                    } else {
                        Vec2::X
                    };
                    new_projectiles.push(ProjectileEntity {
                        id: 0,
                        pos: boss_pos,
                        vel: base_dir * (260.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::True,
                        armor_penetration: 0.70,
                        lifetime: 2.5,
                        radius: 20.0,
                        pierce: 15,
                        faction: Faction::Boss,
                        homing_strength: 0.0,
                        source_weapon: Some(WeaponType::VoidRift),
                    });
                }

                // 1.40 Коллапс Сверхновой (ЭВОЛЮЦИЯ): черная дыра взрывается каскадом сингулярностей
                WeaponType::SingularityCollapse => {
                    let count = 4 + weapon.bonus_projectiles as usize;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            vel: dir * (280.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.95,
                            lifetime: 3.0,
                            radius: 26.0,
                            pierce: 25,
                            faction: Faction::Boss,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::SingularityCollapse),
                        });
                    }
                    world.spawn_particles(boss_pos, 50, MAGENTA, 320.0);
                }
            }
        }
    }

    world.boss_inventory.weapons = weapons;

    // Пассивная регенерация здоровья от BloodStone
    let hp_regen = world.boss_inventory.get_hp_regen();
    if hp_regen > 0.0 {
        world.boss_health.heal(hp_regen * dt);
    }

    // Применение Луча Смерти по тактической цели
    if death_ray_fire {
        world.push_audio(crate::audio::AudioEvent::LaserBeam);
        let target_dir = if let Some(target_pos) = tactical_target_pos {
            (target_pos - boss_pos).normalize_or_zero()
        } else {
            Vec2::new(1.0, 0.0)
        };

        let beam_end = boss_pos + target_dir * 1200.0;
        world.death_ray_active = Some((boss_pos, beam_end, 0.35));

        let ray_norm = target_dir;
        for mob in &mut world.mobs {
            let to_mob = mob.pos - boss_pos;
            let projection = to_mob.dot(ray_norm);
            if projection > 0.0 && projection < 1200.0 {
                let closest_pt = boss_pos + ray_norm * projection;
                if mob.pos.distance_squared(closest_pt) < (mob.radius + 28.0).powi(2) {
                    world.damage_queue.push(DamagePacket {
                        source_id: Some(0),
                        target_id: mob.id,
                        amount: death_ray_damage,
                        damage_type: DamageType::True,
                        armor_penetration: 1.0,
                    });
                }
            }
        }

        // Луч Смерти прожигает и Алтари Крови
        for st in &world.structures {
            let to_st = st.pos - boss_pos;
            let projection = to_st.dot(ray_norm);
            if projection > 0.0 && projection < 1200.0 {
                let closest_pt = boss_pos + ray_norm * projection;
                if st.pos.distance_squared(closest_pt) < (st.radius + 30.0).powi(2) {
                    world.damage_queue.push(DamagePacket {
                        source_id: Some(0),
                        target_id: st.id,
                        amount: death_ray_damage * 1.5,
                        damage_type: DamageType::True,
                        armor_penetration: 1.0,
                    });
                }
            }
        }
    }

    // Модификаторы снарядов от пассивок и реликвий
    let has_colossal_eye = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::ColossalEye);
    let has_archmage_shard = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::ArchmageShard);
    let has_solar_relic = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::SolarRelic);

    // Аура Солнечного Венца (SolarRelic): наносит 40 святого урона/сек вокруг босса
    if has_solar_relic {
        let solar_rad = 180.0;
        let nearby = world.spatial_grid.query_nearby(boss_pos, solar_rad);
        for idx in nearby {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(boss_pos) < solar_rad * solar_rad {
                    world.damage_queue.push(DamagePacket {
                        source_id: Some(0),
                        target_id: mob.id,
                        amount: 40.0 * dt,
                        damage_type: DamageType::Holy,
                        armor_penetration: 0.3,
                    });
                }
            }
        }
    }

    // Присваиваем ID и применяем эффекты реликвий и автонаведения
    let has_blessed_heart = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::BlessedHeart);

    for proj in &mut new_projectiles {
        proj.id = world.alloc_id();

        // 1. Врождённое самонаведение специфических типов оружия
        match proj.source_weapon {
            Some(WeaponType::MissileSwarm) => proj.homing_strength = 7.5,
            Some(WeaponType::DoomsdayBarrage) => proj.homing_strength = 9.5,
            Some(WeaponType::SoulPhantoms) => proj.homing_strength = 6.0,
            Some(WeaponType::BansheeWail) => proj.homing_strength = 8.0,
            Some(WeaponType::MagicWand) => proj.homing_strength = 4.5,
            _ => {}
        }

        // 2. Реликвия "Благословенное Сердце": наделяет АБСОЛЮТНО ВСЕ снаряды босса мощным самонаведением!
        if has_blessed_heart && proj.faction == Faction::Boss {
            proj.homing_strength = proj.homing_strength.max(6.5);
        }

        if has_colossal_eye {
            proj.radius *= 1.5;
            proj.damage *= 2.0;
            proj.pierce += 99;
        }
        if has_archmage_shard {
            proj.pierce += 1;
        }
    }
    world.projectiles.extend(new_projectiles);
    world.acid_pools.extend(new_acid_pools);

    if world.active_boss_id == 1 {
        if world.active_boss_id == 1 {
            update_world_structures(world, dt);
        }
    }
}

/// Updates ranged attacks once per world tick and aims each squad at its assigned Boss.
/// Altar-spawned units have no assigned player and choose the nearest living Boss.
pub fn update_enemy_mob_attacks(world: &mut World, dt: f32) {
    let primary_pos = world.boss_pos;
    let primary_alive = world.boss_health.is_alive();
    let coop_bosses = &world.coop_bosses;
    let mut enemy_shots = Vec::new();

    for mob in &mut world.mobs {
        if !mob.health.is_alive() { continue; }
        let assigned_target = mob.target_boss_id.and_then(|id| {
            if id == 1 && primary_alive {
                Some(primary_pos)
            } else {
                coop_bosses.iter()
                    .find(|player| player.id == id && player.boss_health.is_alive())
                    .map(|player| player.boss_pos)
            }
        });
        let mut nearest_target = if primary_alive {
            Some((primary_pos.distance_squared(mob.pos), primary_pos))
        } else { None };
        for player in coop_bosses.iter().filter(|player| player.boss_health.is_alive()) {
            let distance = player.boss_pos.distance_squared(mob.pos);
            if nearest_target.map_or(true, |(best, _)| distance < best) {
                nearest_target = Some((distance, player.boss_pos));
            }
        }
        let Some(target_pos) = assigned_target.or_else(|| nearest_target.map(|(_, pos)| pos)) else {
            continue;
        };
        let dist_sq = mob.pos.distance_squared(target_pos);
        let shot = if mob.tier == 3 || mob.tier == 11 {
            mob.attack_timer -= dt;
            let range = if mob.tier == 11 { 380.0 } else { 310.0 };
            if dist_sq < range * range && mob.attack_timer <= 0.0 {
                mob.attack_timer = if mob.tier == 11 { 1.6 } else { 2.0 };
                let (damage, damage_type, speed) = if mob.tier == 11 {
                    (48.0, DamageType::True, 260.0)
                } else {
                    (18.0, DamageType::Physical, 230.0)
                };
                Some((mob.pos, target_pos, damage, damage_type, speed, 9.0, None))
            } else { None }
        } else if mob.tier == 7 || mob.tier == 15 {
            mob.attack_timer -= dt;
            let range = if mob.tier == 15 { 440.0 } else { 360.0 };
            if dist_sq < range * range && mob.attack_timer <= 0.0 {
                mob.attack_timer = if mob.tier == 15 { 1.8 } else { 2.2 };
                let (damage, speed) = if mob.tier == 15 { (110.0, 300.0) } else { (35.0, 260.0) };
                Some((mob.pos, target_pos, damage, DamageType::Fire, speed, 14.0, None))
            } else { None }
        } else if mob.tier == 8 || mob.tier == 16 {
            mob.attack_timer -= dt;
            let range = if mob.tier == 16 { 500.0 } else { 420.0 };
            if dist_sq < range * range && mob.attack_timer <= 0.0 {
                mob.attack_timer = if mob.tier == 16 { 2.4 } else { 2.8 };
                let (damage, speed) = if mob.tier == 16 { (160.0, 320.0) } else { (70.0, 280.0) };
                Some((mob.pos, target_pos, damage, DamageType::True, speed, 20.0, Some(WeaponType::ExecutionerAxe)))
            } else { None }
        } else { None };

        if let Some(shot) = shot { enemy_shots.push(shot); }
    }

    for (pos, target_pos, damage, damage_type, speed, radius, source_weapon) in enemy_shots {
        let id = world.alloc_id();
        let direction = (target_pos - pos).normalize_or_zero();
        world.projectiles.push(ProjectileEntity {
            id,
            pos,
            vel: direction * speed,
            damage,
            damage_type,
            armor_penetration: 0.20,
            lifetime: 2.5,
            radius,
            pierce: 1,
            faction: Faction::Director,
            homing_strength: 0.0,
            source_weapon,
        });
    }
}

pub fn update_world_structures(world: &mut World, dt: f32) {
    // Shared world timers run once per frame, while each player gets their own weapon timers.
    // 3. Импульсы, стены и спавн бойцов алтарей
    let mut altar_heal_pulses = Vec::new();
    let mut altar_spawns = Vec::new();

    for structure in &mut world.structures {
        if crate::content::structures::is_altar_structure_type(structure.structure_type) && structure.health.is_alive() {
            structure.pulse_timer += dt;
            if structure.pulse_timer >= 3.0 {
                structure.pulse_timer = 0.0;
                altar_heal_pulses.push(structure.pos);
            }

            structure.spawn_timer += dt;
            let altar_tier = crate::content::structures::altar_tier_from_structure_type(structure.structure_type).unwrap_or(1);
            if structure.spawn_timer >= crate::content::structures::altar_spawn_interval(altar_tier) {
                structure.spawn_timer = 0.0;
                altar_spawns.push((structure.pos, altar_tier));
            }
        } else if structure.structure_type == crate::content::structures::STRUCTURE_WALL && structure.health.is_alive() {
            structure.lifetime -= dt;
            if structure.lifetime <= 0.0 {
                structure.health.current = 0.0;
            }
        }
    }

    // 3.1 Цикл активации Угловых Инфернальных Разломов (раз в 60 секунд)
    world.obelisk_cycle_timer -= dt;
    if world.obelisk_cycle_timer <= 0.0 {
        world.obelisk_cycle_timer = 60.0;
        let obelisk_indices: Vec<usize> = world.structures
            .iter()
            .enumerate()
            .filter(|(_, s)| s.structure_type == crate::content::structures::STRUCTURE_OBELISK && !s.is_active)
            .map(|(i, _)| i)
            .collect();

        if !obelisk_indices.is_empty() {
            let choice = rand::gen_range(0, obelisk_indices.len());
            let target_idx = obelisk_indices[choice];
            if let Some(obelisk) = world.structures.get_mut(target_idx) {
                obelisk.is_active = true;
                obelisk.state_timer = 20.0;
                obelisk.health.current = obelisk.health.max;
                let pos = obelisk.pos;

                let corner_name = if pos.x < 1000.0 && pos.y < 1000.0 {
                    "Северо-Запад"
                } else if pos.x > 1000.0 && pos.y < 1000.0 {
                    "Северо-Восток"
                } else if pos.x < 1000.0 && pos.y > 1000.0 {
                    "Юго-Запад"
                } else {
                    "Юго-Восток"
                };

                world.screen_shake = 16.0;
                world.add_floating_text(
                    pos,
                    format!("⚡ РАЗЛОМ [{}]! 20с до Прорыва!", corner_name),
                    GOLD,
                    3.0,
                );

                // Спавн элитных стражей Разлома вокруг активированного обелиска
                let guard_tier = match world.boss_player_count() {
                    1 => 4, // Голем (Tier 4)
                    2 => 6, // Демон теней (Tier 6)
                    _ => 8, // Жнец Смерти (Tier 8)
                };
                let guard_count = (2 * world.boss_player_count()).clamp(2, 8);
                for i in 0..guard_count {
                    let angle = (i as f32 / guard_count as f32) * std::f32::consts::TAU;
                    let guard_pos = pos + Vec2::new(angle.cos(), angle.sin()) * 115.0;
                    crate::content::mobs::spawn_elite_guard_at_pos(world, guard_tier, guard_pos);
                }
            }
        }
    }

    // 3.2 Таймер зарядки активного разлома и Прорыв
    let mut breach_event_pos = None;
    for st in &mut world.structures {
        if st.structure_type == crate::content::structures::STRUCTURE_OBELISK && st.is_active {
            st.state_timer -= dt;
            if st.state_timer <= 0.0 {
                st.is_active = false;
                breach_event_pos = Some(st.pos);
            }
        }
    }

    // ИНФЕРНАЛЬНЫЙ ПРОРЫВ: призыв элитного легиона по формуле (доход игрока * 15)
    if let Some(pos) = breach_event_pos {
        world.screen_shake = 22.0;
        world.spawn_particles(pos, 50, PURPLE, 240.0);
        world.spawn_particles(pos, 35, RED, 180.0);
        world.add_floating_text(pos, "💀 ИНФЕРНАЛЬНЫЙ ПРОРЫВ! ОРДА ВЫРВАЛАСЬ!".to_string(), RED, 2.5);

        let income = if let Some(d) = world.directors.first() {
            d.income_rate + (world.structures.iter().filter(|s| crate::content::structures::is_altar_structure_type(s.structure_type) && s.health.is_alive()).count() as f32 * 3.0)
        } else {
            12.0
        };
        let mut budget = income * 15.0 * world.boss_player_count() as f32;
        let current_spent = world.directors.first().map_or(0.0, |d| d.total_points_spent);

        while budget >= crate::content::mobs::get_squad_config(1).cost {
            // Spend the breach budget on the most expensive mob that is both
            // unlocked and affordable. This covers all 16 mob types and prevents
            // the previous hard-coded tier-8 ceiling from starving late-game waves.
            let candidate_tier = (1u8..=16).rev().find(|tier| {
                let config = crate::content::mobs::get_squad_config(*tier);
                current_spent >= config.unlock_spent_threshold && budget >= config.cost
            });
            let Some(candidate_tier) = candidate_tier else { break; };
            let cost = crate::content::mobs::get_squad_config(candidate_tier).cost;
            let offset = Vec2::new(rand::gen_range(-90.0, 90.0), rand::gen_range(-90.0, 90.0));
            crate::content::mobs::spawn_single_mob_at_pos(world, candidate_tier, pos + offset);
            budget -= cost;
        }
    }

    // Each altar's reinforcement wave scales with the number of Boss players.
    for (altar_pos, altar_tier) in altar_spawns {
        for _ in 0..world.boss_player_count() {
            let offset = Vec2::new(rand::gen_range(-25.0, 25.0), rand::gen_range(-25.0, 25.0));
            crate::content::mobs::spawn_single_mob_at_pos(world, altar_tier, altar_pos + offset);
        }
        if !world.is_headless {
            world.spawn_particles(altar_pos, 5, Color::new(0.6, 0.1, 0.8, 0.7), 40.0);
        }
    }

    // Лечебный импульс (+35 HP союзным мобам каждые 3 секунды)
    for altar_pos in altar_heal_pulses {
        let nearby = world.spatial_grid.query_nearby(altar_pos, 180.0);
        for idx in nearby {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(altar_pos) < 180.0 * 180.0 {
                    mob.health.heal(35.0);
                }
            }
        }
        world.spawn_particles(altar_pos, 20, PURPLE, 120.0);
    }
}

