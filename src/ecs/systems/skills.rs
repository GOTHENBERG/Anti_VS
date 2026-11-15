use macroquad::prelude::*;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::{AcidPoolEntity, ProjectileEntity, World};

/// Умный тактический выбор цели для направленных атак босса.
/// Оценивает разрушаемые Алтари Крови, скопления опасных культистов-стрелков,
/// камикадзе и непосредственные угрозы вблизи.
pub fn select_tactical_target(world: &World, boss_pos: Vec2, max_range: f32) -> Option<Vec2> {
    let mut best_target = None;
    let mut best_score = -1000.0f32;

    // 1. Оценка Алтарей Крови (Высокий стратегический приоритет для уничтожения)
    for st in &world.structures {
        let dist = st.pos.distance(boss_pos);
        if dist <= max_range {
            let score = 160.0 - dist * 0.12;
            if score > best_score {
                best_score = score;
                best_target = Some(st.pos);
            }
        }
    }

    // 2. Оценка мобов через Spatial Grid
    let nearby_indices = world.spatial_grid.query_nearby(boss_pos, max_range);

    // Подсчет позиций культистов для выявления скоплений стрелков
    let cultist_positions: Vec<Vec2> = nearby_indices.iter()
        .filter_map(|&idx| world.mobs.get(idx))
        .filter(|m| m.tier == 3)
        .map(|m| m.pos)
        .collect();

    for &idx in &nearby_indices {
        if let Some(mob) = world.mobs.get(idx) {
            let dist = mob.pos.distance(boss_pos);
            if dist > max_range {
                continue;
            }

            let mut score = match mob.tier {
                // Тир 3 (Культисты-стрелки): опаснейшие дальники! Приоритетнейшая цель для снайпинга!
                3 => {
                    let mut s = 140.0;
                    let nearby_cultists = cultist_positions.iter()
                        .filter(|&&cp| cp.distance_squared(mob.pos) < 130.0 * 130.0)
                        .count();
                    s += (nearby_cultists as f32) * 22.0;
                    s
                }
                // Тир 5 (Камикадзе): взрывоопасны при приближении
                5 => {
                    if dist < 150.0 { 135.0 } else { 70.0 }
                }
                // Тир 2 (Гончие): скоростные фланкеры
                2 => 65.0,
                // Тир 4 (Големы): танки
                4 => 40.0,
                // Тир 1 (Скелеты): пушечное мясо
                _ => {
                    if dist < 55.0 { 85.0 } else { 15.0 }
                }
            };

            // Добивание раненых
            if mob.health.current < mob.health.max * 0.35 {
                score += 35.0;
            }

            // Штраф за дальность
            score -= dist * 0.14;

            if score > best_score {
                best_score = score;
                best_target = Some(mob.pos);
            }
        }
    }

    best_target
}

pub fn update_skills(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let cd_multiplier = world.boss_inventory.get_cooldown_multiplier();
    let armor_pen = world.boss_inventory.get_armor_penetration();
    let dmg_mult = world.boss_inventory.get_damage_multiplier();
    let spd_mult = world.boss_inventory.get_projectile_speed_multiplier();

    // Вращение орбит для Бури Клинков и Праведного Щита
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
                    let offset = if let Some(target_pos) = tactical_target_pos {
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
                            source_weapon: Some(WeaponType::BloodDaggers),
                        });
                    }
                }

                // 1.20 Кровавая Жатва (ЭВОЛЮЦИЯ): веер кинжалов с вампиризмом
                WeaponType::BloodHarvest => {
                    let count = 8;
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
                            source_weapon: Some(WeaponType::BloodHarvest),
                        });
                    }
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

    // Присваиваем ID снарядам
    for proj in &mut new_projectiles {
        proj.id = world.alloc_id();
    }
    world.projectiles.extend(new_projectiles);
    world.acid_pools.extend(new_acid_pools);

    // 2. Стрельба культистов (Тир 3), Огненных элементалей (Тир 7) и Жнеца Смерти (Тир 8)
    let mut enemy_shots = Vec::new();
    for mob in &mut world.mobs {
        if mob.tier == 3 {
            mob.attack_timer -= dt;
            let dist_sq = mob.pos.distance_squared(boss_pos);
            if dist_sq < 310.0 * 310.0 && mob.attack_timer <= 0.0 {
                mob.attack_timer = 2.0;
                enemy_shots.push((mob.pos, 18.0, DamageType::Physical, 230.0, 8.0, None));
            }
        } else if mob.tier == 7 {
            mob.attack_timer -= dt;
            let dist_sq = mob.pos.distance_squared(boss_pos);
            if dist_sq < 360.0 * 360.0 && mob.attack_timer <= 0.0 {
                mob.attack_timer = 2.2;
                enemy_shots.push((mob.pos, 35.0, DamageType::Fire, 260.0, 12.0, None));
            }
        } else if mob.tier == 8 {
            mob.attack_timer -= dt;
            let dist_sq = mob.pos.distance_squared(boss_pos);
            if dist_sq < 420.0 * 420.0 && mob.attack_timer <= 0.0 {
                mob.attack_timer = 2.8;
                enemy_shots.push((mob.pos, 70.0, DamageType::True, 280.0, 18.0, Some(WeaponType::ExecutionerAxe)));
            }
        }
    }

    for (pos, dmg, dmg_type, speed, rad, source_w) in enemy_shots {
        let id = world.alloc_id();
        let dir = (boss_pos - pos).normalize_or_zero();
        world.projectiles.push(ProjectileEntity {
            id,
            pos,
            vel: dir * speed,
            damage: dmg,
            damage_type: dmg_type,
            armor_penetration: 0.20,
            lifetime: 2.5,
            radius: rad,
            pierce: 1,
            faction: Faction::Director,
            source_weapon: source_w,
        });
    }

    // 3. Импульсы Алтарей Крови
    let mut altar_pulses = Vec::new();
    for structure in &mut world.structures {
        structure.pulse_timer += dt;
        if structure.pulse_timer >= 3.0 {
            structure.pulse_timer = 0.0;
            altar_pulses.push(structure.pos);
        }
    }

    for altar_pos in altar_pulses {
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

