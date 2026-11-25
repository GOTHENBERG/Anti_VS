use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::{AcidPoolEntity, ProjectileEntity, World};

/// Выбор ближайшей цели для направленных атак босса.
/// Находит ближайшего живого врага (моба) или разрушаемую структуру в радиусе max_range.
pub fn select_nearest_target(world: &World, boss_pos: Vec2, max_range: f32) -> Option<Vec2> {
    let mut nearest_target = None;
    let mut min_dist_sq = max_range * max_range;

    // 1. Поиск ближайшего живого противника через Spatial Grid O(1)
    let nearby_indices = world.spatial_grid.query_nearby(boss_pos, max_range);
    for &idx in &nearby_indices {
        if let Some(mob) = world.mobs.get(idx) {
            if mob.health.is_alive() {
                let d_sq = mob.pos.distance_squared(boss_pos);
                if d_sq < min_dist_sq {
                    min_dist_sq = d_sq;
                    nearest_target = Some(mob.pos);
                }
            }
        }
    }

    // 2. Разрушаемые структуры (Алтари, Стены, Обелиски), если они ближе или рядом нет мобов
    for st in &world.structures {
        if st.health.is_alive() && crate::content::structures::boss_can_damage_structure(st) {
            let d_sq = st.pos.distance_squared(boss_pos);
            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
                nearest_target = Some(st.pos);
            }
        }
    }

    nearest_target
}

/// Алиас для обратной совместимости
#[inline]
pub fn select_tactical_target(world: &World, boss_pos: Vec2, max_range: f32) -> Option<Vec2> {
    select_nearest_target(world, boss_pos, max_range)
}

/// Находит точку скопления вражеской массы (центр кластера) в пределах радиуса стрельбы.
/// Для AoE-оружий (Метеоры, Сингулярность, Разлом Бездны, Ядовитые споры) целится в ближайшее
/// наибольшее скопление противников без искусственных приоритетов.
pub fn select_cluster_density_target(world: &World, boss_pos: Vec2, max_range: f32, aoe_radius: f32) -> Option<Vec2> {
    if world.crowd_grid.is_active() {
        if let Some((coarse_center, _, _)) = world.crowd_grid.find_peak_density_cell(boss_pos, max_range, aoe_radius) {
            if world.spatial_grid.is_empty() {
                return Some(coarse_center);
            }

            // 2-шаговое Mean-Shift уточнение внутри окна AoE без единой аллокации:
            // Шаг 0 стягивает фокус с центра квадрата 128x128 на плотное ядро (в т.ч. на границах ячеек),
            // Шаг 1 точно центрирует круг поражения `aoe_radius` на центре масс мобов.
            let aoe_radius_sq = aoe_radius * aoe_radius;
            let expanded_rad = aoe_radius + world.crowd_grid.cell_size * 0.5;
            let mut center = coarse_center;

            for step in 0..2 {
                let (query_r, query_r_sq) = if step == 0 {
                    (expanded_rad, expanded_rad * expanded_rad)
                } else {
                    (aoe_radius, aoe_radius_sq)
                };
                let pad = Vec2::splat(query_r);
                let mut sum_pos = Vec2::ZERO;
                let mut count = 0.0f32;

                world.spatial_grid.query_aabb_unique(center - pad, center + pad, |m_idx| {
                    if let Some(m) = world.mobs.get(m_idx) {
                        if m.health.is_alive() && m.pos.distance_squared(center) <= query_r_sq {
                            sum_pos += m.pos;
                            count += 1.0;
                        }
                    }
                });

                if count > 0.0 {
                    let refined = sum_pos / count;
                    if refined.distance(boss_pos) <= max_range + aoe_radius * 0.25 {
                        center = refined;
                    }
                }
            }

            return Some(center);
        }
    }

    select_nearest_target(world, boss_pos, max_range)
}

pub fn update_skills(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let cd_multiplier = world.boss_inventory.get_cooldown_multiplier();
    let armor_pen = world.boss_inventory.get_armor_penetration();
    let dmg_mult = world.boss_inventory.get_damage_multiplier();
    let spd_mult = world.boss_inventory.get_projectile_speed_multiplier();
    let aoe_mult = world.boss_inventory.get_aoe_multiplier();
    let extra_proj = world.boss_inventory.get_twin_prism_projectiles();
    let owner_boss_id = Some(world.active_boss_id);

    // Вращение орбит для Шквала Лезвий, Бастиона Света и Орбитального Гримуара
    // Shared match time advances once, even when co-op actors are evaluated through
    // the legacy single-Boss context swap.
    if world.active_boss_id == 1 {
        world.game_time += dt;
    }
    world.blade_squall_angle = (world.blade_squall_angle + dt * 6.0) % (std::f32::consts::PI * 2.0);
    world.light_bastion_angle = (world.light_bastion_angle + dt * 4.0) % (std::f32::consts::PI * 2.0);
    world.boss_inventory.scriptures_orbit_angle = (world.boss_inventory.scriptures_orbit_angle + dt * 4.5) % (std::f32::consts::PI * 2.0);

    // Пассивная перезарядка зарядов Астрального Савана / Савана Кровавого Императора
    let has_cloak = world.boss_inventory.has_weapon(WeaponType::AstralCloak);
    let has_shroud = world.boss_inventory.has_weapon(WeaponType::BloodEmperorShroud);
    if has_cloak || has_shroud {
        let max_charges = if has_shroud { 4 } else { 3 };
        let recharge_cd = if has_shroud { 6.0 } else { 10.0 };
        world.boss_inventory.astral_cloak_timer += dt;
        if world.boss_inventory.astral_cloak_timer >= recharge_cd {
            world.boss_inventory.astral_cloak_timer = 0.0;
            if world.boss_inventory.astral_cloak_charges < max_charges {
                world.boss_inventory.astral_cloak_charges += 1;
            }
        }
    }

    // Таймер активного Луча Смерти
    if let Some((_, _, ref mut timer)) = world.death_ray_active {
        *timer -= dt;
        if *timer <= 0.0 {
            world.death_ray_active = None;
        }
    }

    // Выбор ближайшей цели для направленных атак босса (без тактических приоритетов)
    let tactical_target_pos = select_nearest_target(world, boss_pos, 380.0);
    // Кластерное наведение по центру массы толпы для разрушительных AoE атак
    let cluster_target_pos = select_cluster_density_target(world, boss_pos, 420.0, 130.0);

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
                WeaponType::WindBlade | WeaponType::BladeSquall | WeaponType::ExecutionerAxe |
                WeaponType::WhirlwindCleaver | WeaponType::BloodDaggers | WeaponType::BloodHarvest => {
                    Some(crate::audio::AudioEvent::BladeSlash)
                }
                WeaponType::FireAura | WeaponType::NetherBlaze | WeaponType::PoisonOrbs | WeaponType::ToxicPestilence => {
                    Some(crate::audio::AudioEvent::FireBlast)
                }
                WeaponType::ArcaneScepter | WeaponType::HolyHammer | WeaponType::LightBastion |
                WeaponType::CelestialBow | WeaponType::StarfallBarrage => {
                    Some(crate::audio::AudioEvent::ArcaneScepter)
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
                    Some(crate::audio::AudioEvent::ArcaneScepter)
                }
                WeaponType::FallingLuminary | WeaponType::ArmageddonStorm => {
                    Some(crate::audio::AudioEvent::FireBlast)
                }
                WeaponType::VoidRift | WeaponType::SingularityCollapse => {
                    Some(crate::audio::AudioEvent::LaserBeam)
                }
                WeaponType::SacredScriptures | WeaponType::InfernalLiturgy => {
                    Some(crate::audio::AudioEvent::ArcaneScepter)
                }
                WeaponType::FaithBoomerang | WeaponType::HeavenlyJudgment => {
                    Some(crate::audio::AudioEvent::BladeSlash)
                }
                WeaponType::CursedChalice | WeaponType::BloodDeluge => {
                    Some(crate::audio::AudioEvent::FireBlast)
                }
                WeaponType::VampiricAura | WeaponType::SoulFeast => {
                    Some(crate::audio::AudioEvent::ArcaneScepter)
                }
                WeaponType::RunicPrism | WeaponType::SpatialShatter => {
                    Some(crate::audio::AudioEvent::LaserBeam)
                }
                WeaponType::AbyssalBeam | WeaponType::ApocalypseCascade => {
                    Some(crate::audio::AudioEvent::LaserBeam)
                }
                WeaponType::ChronoStasis | WeaponType::InfiniteDilation => {
                    Some(crate::audio::AudioEvent::IceFreeze)
                }
                WeaponType::AstralCloak | WeaponType::BloodEmperorShroud => {
                    Some(crate::audio::AudioEvent::BladeSlash)
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
                            prev_pos: boss_pos,
                            vel: dir * (320.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen,
                            lifetime: 0.75,
                            radius: 12.0,
                            pierce: 2,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::WindBlade),
                        });
                    }
                }

                // 1.2 Шквал Лезвий (ЭВОЛЮЦИЯ): непрерывный круговой вихрь клинков
                WeaponType::BladeSquall => {}

                // 1.3 Огненная Аура: волна огня в радиусе 125px
                WeaponType::FireAura => {
                    let aura_radius = 125.0 + (weapon.level as f32 * 5.0);
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, aura_radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < aura_radius * aura_radius {
                                mob.status.apply_burn(3.0, effective_damage * 0.4);
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    effective_damage,
                                    DamageType::Fire,
                                    armor_pen,
                                ).with_owner(owner_boss_id));
                            }
                        }
                    }

                    // Аура наносит урон и разрушаемым Алтарям Крови
                    for st in &world.structures {
                        if st.pos.distance_squared(boss_pos) < (aura_radius + st.radius).powi(2) {
                            world.damage_queue.push(DamagePacket::structure(
                                Some(0),
                                st.id,
                                effective_damage * 0.8,
                                DamageType::Fire,
                                armor_pen,
                            ).with_owner(owner_boss_id));
                        }
                    }

                    world.spawn_particles(boss_pos, 25, ORANGE, 150.0);
                }

                // 1.4 Пекло Преисподней (ЭВОЛЮЦИЯ): процентный урон по площади + взрыв трупов
                WeaponType::NetherBlaze => {
                    let aura_radius = 175.0;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, aura_radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < aura_radius * aura_radius {
                                let percent_dmg = mob.health.max * 0.10 + effective_damage;
                                mob.status.apply_burn(4.0, effective_damage * 0.6);
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    percent_dmg,
                                    DamageType::Fire,
                                    armor_pen + 0.25,
                                ).with_owner(owner_boss_id));
                            }
                        }
                    }

                    for st in &world.structures {
                        if st.pos.distance_squared(boss_pos) < (aura_radius + st.radius).powi(2) {
                            world.damage_queue.push(DamagePacket::structure(
                                Some(0),
                                st.id,
                                effective_damage * 1.3,
                                DamageType::Fire,
                                armor_pen + 0.25,
                            ).with_owner(owner_boss_id));
                        }
                    }

                    world.spawn_particles(boss_pos, 45, RED, 190.0);
                }

                // 1.5 Скипетр Арканы: быстрые стрелы в тактическую цель
                WeaponType::ArcaneScepter => {
                    if let Some(target_pos) = tactical_target_pos {
                        let dir = (target_pos - boss_pos).normalize_or_zero();
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos,
                            prev_pos: boss_pos,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 1.0,
                            lifetime: 0.75,
                            radius: 8.0,
                            pierce: 1 + (weapon.level / 3) as u8,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::ArcaneScepter),
                        });
                    }
                }

                // 1.6 Луч Смерти (ЭВОЛЮЦИЯ): прожигающий лазер через весь экран
                WeaponType::DeathRay => {
                    death_ray_fire = true;
                    death_ray_damage = effective_damage;
                }

                // 1.7 Святой Молот (ОРБИТАЛЬНЫЙ НАВЫК): непрерывно вращающиеся освященные молоты вокруг босса
                WeaponType::HolyHammer => {
                    world.spawn_particles(boss_pos, 8, GOLD, 90.0);
                }

                // 1.8 Бастион Света (ЭВОЛЮЦИЯ, ОРБИТАЛЬНЫЙ НАВЫК): священные щиты с вампиризмом
                WeaponType::LightBastion => {
                    world.spawn_particles(boss_pos, 12, GOLD, 110.0);
                }

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
                            prev_pos: boss_pos,
                            vel: dir * (350.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Ice,
                            armor_penetration: armor_pen,
                            lifetime: 0.85,
                            radius: 9.0,
                            pierce: 2,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    effective_damage,
                                    DamageType::Ice,
                                    armor_pen,
                                ).with_owner(owner_boss_id));
                            }
                        }
                    }

                    for st in &world.structures {
                        if st.pos.distance_squared(boss_pos) < (radius + st.radius).powi(2) {
                            world.damage_queue.push(DamagePacket::structure(
                                Some(0),
                                st.id,
                                effective_damage,
                                DamageType::Ice,
                                armor_pen,
                            ).with_owner(owner_boss_id));
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
                        owner_boss_id,
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
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (310.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.15,
                            lifetime: 1.8,
                            radius: 14.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (380.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.35,
                            lifetime: 2.2,
                            radius: 18.0,
                            pierce: 8,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (440.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen + 0.20,
                            lifetime: 0.80,
                            radius: 10.0,
                            pierce: 3,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (620.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen + 0.40,
                            lifetime: 2.2,
                            radius: 13.0,
                            pierce: 6,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 1.0,
                            lifetime: 1.6,
                            radius: 12.0,
                            pierce: 3,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                                    world.damage_queue.push(DamagePacket::mob(
                                        Some(0),
                                        idx,
                                        mob.id,
                                        effective_damage,
                                        DamageType::True,
                                        1.0,
                                    ).with_owner(owner_boss_id));
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
                            prev_pos: boss_pos,
                            vel: dir * (380.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen,
                            lifetime: 0.75,
                            radius: 8.0,
                            pierce: 1,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.25,
                            lifetime: 0.80,
                            radius: 10.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                        prev_pos: boss_pos,
                        vel: base_dir * (220.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::True,
                        armor_penetration: 0.8,
                        lifetime: 1.1,
                        radius: 18.0,
                        pierce: 10,
                        faction: Faction::Boss,
                        owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (220.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 1.0,
                            lifetime: 1.2,
                            radius: 28.0,
                            pierce: 20,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (340.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Ice,
                            armor_penetration: armor_pen,
                            lifetime: 1.0,
                            radius: 12.0,
                            pierce: 6,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (360.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Ice,
                            armor_penetration: armor_pen + 0.3,
                            lifetime: 1.0,
                            radius: 16.0,
                            pierce: 12,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                        prev_pos: boss_pos,
                        vel: base_dir * (550.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::Fire,
                        armor_penetration: 0.5,
                        lifetime: 0.65,
                        radius: 20.0,
                        pierce: 15,
                        faction: Faction::Boss,
                        owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (650.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Fire,
                            armor_penetration: 0.75,
                            lifetime: 1.8,
                            radius: 24.0,
                            pierce: 25,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (360.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.25,
                            lifetime: 2.2,
                            radius: 11.0,
                            pierce: 2,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.50,
                            lifetime: 2.5,
                            radius: 15.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: base_dir * (500.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.50,
                            lifetime: 1.6,
                            radius: 12.0,
                            pierce: 4,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (580.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.80,
                            lifetime: 2.0,
                            radius: 16.0,
                            pierce: 8,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    effective_damage,
                                    DamageType::Holy,
                                    armor_pen + 0.35,
                                ).with_owner(owner_boss_id));
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
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    effective_damage,
                                    DamageType::Holy,
                                    armor_pen + 0.60,
                                ).with_owner(owner_boss_id));
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
                            prev_pos: boss_pos,
                            vel: dir * (320.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.40,
                            lifetime: 2.8,
                            radius: 14.0,
                            pierce: 5,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (360.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.70,
                            lifetime: 3.5,
                            radius: 18.0,
                            pierce: 12,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::BansheeWail),
                        });
                    }
                    world.spawn_particles(boss_pos, 40, Color::new(0.4, 1.0, 0.7, 0.9), 240.0);
                }

                // 1.35 Орбитальные Бастионы (ОРБИТАЛЬНЫЙ НАВЫК): непрерывно вращающиеся защитные бастионы
                WeaponType::AegisOrbiters => {
                    world.spawn_particles(boss_pos, 10, Color::new(0.3, 0.9, 1.0, 0.9), 110.0);
                }

                // 1.36 Несокрушимая Крепость (ЭВОЛЮЦИЯ, ОРБИТАЛЬНЫЙ НАВЫК): гигантские крепостные щиты с защитным полем
                WeaponType::InvincibleBastion => {
                    world.spawn_particles(boss_pos, 18, Color::new(0.35, 0.95, 1.0, 0.95), 130.0);
                }

                // 1.37 Падающее Светило: падающий метеорит с колоссальным взрывом
                WeaponType::FallingLuminary => {
                    let target = cluster_target_pos.unwrap_or(boss_pos + Vec2::new(rand::gen_range(-150.0, 150.0), rand::gen_range(-150.0, 150.0)));
                    let dir = (target - boss_pos).normalize_or_zero();
                    new_projectiles.push(ProjectileEntity {
                        id: 0,
                        pos: boss_pos,
                        prev_pos: boss_pos,
                        vel: dir * (400.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::Fire,
                        armor_penetration: 0.50,
                        lifetime: 1.5,
                        radius: 24.0,
                        pierce: 1,
                        faction: Faction::Boss,
                        owner_boss_id,
                        homing_strength: 0.0,
                        source_weapon: Some(WeaponType::FallingLuminary),
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
                            prev_pos: boss_pos,
                            vel: dir * (450.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Fire,
                            armor_penetration: 0.75,
                            lifetime: 1.6,
                            radius: 28.0,
                            pierce: 1,
                            faction: Faction::Boss,
                            owner_boss_id,
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
                        prev_pos: boss_pos,
                        vel: base_dir * (260.0 * spd_mult),
                        damage: effective_damage,
                        damage_type: DamageType::True,
                        armor_penetration: 0.70,
                        lifetime: 2.5,
                        radius: 20.0,
                        pierce: 15,
                        faction: Faction::Boss,
                        owner_boss_id,
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
                            prev_pos: boss_pos,
                            vel: dir * (280.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::True,
                            armor_penetration: 0.95,
                            lifetime: 3.0,
                            radius: 26.0,
                            pierce: 25,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::SingularityCollapse),
                        });
                    }
                    world.spawn_particles(boss_pos, 50, MAGENTA, 320.0);
                }

                // 1.41 Орбитальный Гримуар (ОРБИТАЛЬНЫЙ НАВЫК): священные фолианты непрерывно кружатся вокруг босса
                WeaponType::SacredScriptures => {
                    world.spawn_particles(boss_pos, 16, GOLD, 100.0);
                }

                // 1.42 Инфернальная Литургия (ЭВОЛЮЦИЯ, ОРБИТАЛЬНЫЙ НАВЫК): пылающие адские скрижали
                WeaponType::InfernalLiturgy => {
                    world.spawn_particles(boss_pos, 28, ORANGE, 125.0);
                }

                // 1.43 Клинок Веры: крестообразный бумеранг, прорезающий ряды врагов и возвращающийся назад
                WeaponType::FaithBoomerang => {
                    let base_dir = tactical_target_pos
                        .map(|tp| (tp - boss_pos).normalize_or_zero())
                        .unwrap_or(Vec2::new(1.0, 0.0));
                    let count = 1 + weapon.bonus_projectiles as usize + extra_proj;
                    for i in 0..count {
                        let spread = if count > 1 {
                            (i as f32 - (count - 1) as f32 / 2.0) * 0.25
                        } else { 0.0 };
                        let angle = base_dir.y.atan2(base_dir.x) + spread;
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos + dir * 25.0,
                            prev_pos: boss_pos + dir * 25.0,
                            vel: dir * (360.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Physical,
                            armor_penetration: armor_pen + 0.30,
                            lifetime: 2.8,
                            radius: 17.0 * aoe_mult,
                            pierce: 25,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::FaithBoomerang),
                        });
                    }
                }

                // 1.44 Небесный Вердикт (ЭВОЛЮЦИЯ): 4 священных клинка во все стороны с гарантированным критом
                WeaponType::HeavenlyJudgment => {
                    let count = 4 + weapon.bonus_projectiles as usize + extra_proj;
                    for i in 0..count {
                        let angle = (i as f32) * std::f32::consts::PI * 2.0 / (count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos + dir * 30.0,
                            prev_pos: boss_pos + dir * 30.0,
                            vel: dir * (420.0 * spd_mult),
                            damage: effective_damage * 1.4,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen + 0.50,
                            lifetime: 3.2,
                            radius: 24.0 * aoe_mult,
                            pierce: 99,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::HeavenlyJudgment),
                        });
                    }
                    world.spawn_particles(boss_pos, 40, YELLOW, 260.0);
                }

                // 1.45 Осквернённая Чаша: бросок кубка с разливающейся лужей священного пламени
                WeaponType::CursedChalice => {
                    let count = 1 + weapon.bonus_projectiles as usize + extra_proj;
                    for _ in 0..count {
                        let offset = if let Some(target_pos) = cluster_target_pos {
                            let jitter = Vec2::new(crate::core::rand::gen_range(-40.0, 40.0), crate::core::rand::gen_range(-40.0, 40.0));
                            (target_pos - boss_pos + jitter).clamp_length_max(320.0)
                        } else {
                            Vec2::new(crate::core::rand::gen_range(-140.0, 140.0), crate::core::rand::gen_range(-140.0, 140.0))
                        };
                        new_acid_pools.push(AcidPoolEntity {
                            pos: boss_pos + offset,
                            radius: 65.0 * aoe_mult,
                            lifetime: 4.5,
                            dps: effective_damage * 0.85,
                            corrosive: false,
                            is_director_source: false,
                            owner_boss_id,
                        });
                    }
                }

                // 1.46 Кровавый Потоп (ЭВОЛЮЦИЯ): каскад огромных кровавых гейзеров с вампиризмом
                WeaponType::BloodDeluge => {
                    let count = 2 + weapon.bonus_projectiles as usize + extra_proj;
                    for _ in 0..count {
                        let offset = if let Some(target_pos) = cluster_target_pos {
                            let jitter = Vec2::new(crate::core::rand::gen_range(-60.0, 60.0), crate::core::rand::gen_range(-60.0, 60.0));
                            (target_pos - boss_pos + jitter).clamp_length_max(380.0)
                        } else {
                            Vec2::new(crate::core::rand::gen_range(-180.0, 180.0), crate::core::rand::gen_range(-180.0, 180.0))
                        };
                        new_acid_pools.push(AcidPoolEntity {
                            pos: boss_pos + offset,
                            radius: 110.0 * aoe_mult,
                            lifetime: 6.5,
                            dps: effective_damage * 1.25,
                            corrosive: true,
                            is_director_source: false,
                            owner_boss_id,
                        });
                    }
                    world.boss_health.heal(effective_damage * 0.15);
                    world.spawn_particles(boss_pos, 30, RED, 200.0);
                }

                // 1.47 Аура Вампиризма: пульсирующее кольцо, вытягивающее жизнь врагов
                WeaponType::VampiricAura => {
                    let aura_radius = (140.0 + (weapon.level as f32 * 10.0)) * aoe_mult;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, aura_radius);
                    let mut total_drain = 0.0f32;
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < aura_radius * aura_radius {
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    effective_damage,
                                    DamageType::True,
                                    1.0,
                                ).with_owner(owner_boss_id));
                                total_drain += effective_damage * 0.12;
                            }
                        }
                    }
                    if total_drain > 0.0 {
                        world.boss_health.heal(total_drain.min(45.0));
                    }
                    world.spawn_particles(boss_pos, 18, PURPLE, aura_radius);
                }

                // 1.48 Пиршество Душ (ЭВОЛЮЦИЯ): колоссальная аура некротического поглощения
                WeaponType::SoulFeast => {
                    let aura_radius = 230.0 * aoe_mult;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, aura_radius);
                    let mut total_drain = 0.0f32;
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < aura_radius * aura_radius {
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    effective_damage * 1.4,
                                    DamageType::True,
                                    1.0,
                                ).with_owner(owner_boss_id));
                                total_drain += effective_damage * 0.25;
                            }
                        }
                    }
                    if total_drain > 0.0 {
                        world.boss_health.heal(total_drain.min(95.0));
                    }
                    world.spawn_particles(boss_pos, 45, DARKPURPLE, aura_radius);
                }

                // 1.49 Руническая Призма: рикошетящие лучи-осколки
                WeaponType::RunicPrism => {
                    let count = 2 + weapon.bonus_projectiles as usize + extra_proj;
                    let base_angle = crate::core::rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                    for i in 0..count {
                        let angle = base_angle + (i as f32 * std::f32::consts::PI * 2.0 / count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos + dir * 25.0,
                            prev_pos: boss_pos + dir * 25.0,
                            vel: dir * (340.0 * spd_mult),
                            damage: effective_damage,
                            damage_type: DamageType::Holy,
                            armor_penetration: armor_pen + 0.35,
                            lifetime: 1.1,
                            radius: 14.0 * aoe_mult,
                            pierce: 6,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::RunicPrism),
                        });
                    }
                }

                // 1.50 Искривление Пространства (ЭВОЛЮЦИЯ): каскад расщепляющихся призм с чистым уроном
                WeaponType::SpatialShatter => {
                    let count = 4 + weapon.bonus_projectiles as usize + extra_proj;
                    let base_angle = crate::core::rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                    for i in 0..count {
                        let angle = base_angle + (i as f32 * std::f32::consts::PI * 2.0 / count as f32);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        new_projectiles.push(ProjectileEntity {
                            id: 0,
                            pos: boss_pos + dir * 30.0,
                            prev_pos: boss_pos + dir * 30.0,
                            vel: dir * (380.0 * spd_mult),
                            damage: effective_damage * 1.35,
                            damage_type: DamageType::True,
                            armor_penetration: 1.0,
                            lifetime: 1.1,
                            radius: 18.0 * aoe_mult,
                            pierce: 16,
                            faction: Faction::Boss,
                            owner_boss_id,
                            homing_strength: 0.0,
                            source_weapon: Some(WeaponType::SpatialShatter),
                        });
                    }
                    world.spawn_particles(boss_pos, 35, SKYBLUE, 250.0);
                }

                // 1.51 Столп Бездны: вертикальный карающий луч Бездны
                WeaponType::AbyssalBeam => {
                    let count = 1 + weapon.bonus_projectiles as usize + extra_proj;
                    for _ in 0..count {
                        let target = tactical_target_pos.unwrap_or_else(|| {
                            boss_pos + Vec2::new(crate::core::rand::gen_range(-250.0, 250.0), crate::core::rand::gen_range(-250.0, 250.0))
                        });
                        let beam_rad = 60.0 * aoe_mult;
                        let nearby = world.spatial_grid.query_nearby(target, beam_rad);
                        for idx in nearby {
                            if let Some(mob) = world.mobs.get_mut(idx) {
                                if mob.pos.distance_squared(target) < beam_rad * beam_rad {
                                    world.damage_queue.push(DamagePacket::mob(
                                        Some(0),
                                        idx,
                                        mob.id,
                                        effective_damage,
                                        DamageType::True,
                                        1.0,
                                    ).with_owner(owner_boss_id));
                                }
                            }
                        }
                        for st in &world.structures {
                            if st.pos.distance_squared(target) < (st.radius + beam_rad).powi(2) {
                                world.damage_queue.push(DamagePacket::structure(
                                    Some(0),
                                    st.id,
                                    effective_damage * 1.5,
                                    DamageType::True,
                                    1.0,
                                ).with_owner(owner_boss_id));
                            }
                        }
                        world.spawn_particles(target, 40, DARKPURPLE, beam_rad * 1.5);
                    }
                }

                // 1.52 Каскад Апокалипсиса (ЭВОЛЮЦИЯ): 5 сверхмассивных столпов погибели
                WeaponType::ApocalypseCascade => {
                    let count = 5 + weapon.bonus_projectiles as usize + extra_proj;
                    let base_target = cluster_target_pos.unwrap_or(boss_pos);
                    for i in 0..count {
                        let offset = Vec2::new((i as f32 - (count - 1) as f32 / 2.0) * 110.0, crate::core::rand::gen_range(-60.0, 60.0));
                        let strike_pt = base_target + offset;
                        let beam_rad = 85.0 * aoe_mult;
                        let nearby = world.spatial_grid.query_nearby(strike_pt, beam_rad);
                        for idx in nearby {
                            if let Some(mob) = world.mobs.get_mut(idx) {
                                if mob.pos.distance_squared(strike_pt) < beam_rad * beam_rad {
                                    world.damage_queue.push(DamagePacket::mob(
                                        Some(0),
                                        idx,
                                        mob.id,
                                        effective_damage * 1.5,
                                        DamageType::True,
                                        1.0,
                                    ).with_owner(owner_boss_id));
                                }
                            }
                        }
                        for st in &world.structures {
                            if st.pos.distance_squared(strike_pt) < (st.radius + beam_rad).powi(2) {
                                world.damage_queue.push(DamagePacket::structure(
                                    Some(0),
                                    st.id,
                                    effective_damage * 2.0,
                                    DamageType::True,
                                    1.0,
                                ).with_owner(owner_boss_id));
                            }
                        }
                        world.spawn_particles(strike_pt, 50, MAGENTA, beam_rad * 1.8);
                    }
                }

                // 1.53 Хроно-Стазис: кольцо заморозки времени
                WeaponType::ChronoStasis => {
                    let radius = 240.0 * aoe_mult;
                    let mobs_in_range = world.spatial_grid.query_nearby(boss_pos, radius);
                    for idx in mobs_in_range {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.pos.distance_squared(boss_pos) < radius * radius {
                                mob.status.apply_freeze(3.0);
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    effective_damage,
                                    DamageType::Ice,
                                    armor_pen + 0.3,
                                ).with_owner(owner_boss_id));
                            }
                        }
                    }
                    world.spawn_particles(boss_pos, 30, BLUE, radius);
                }

                // 1.54 Бесконечная Дилатация (ЭВОЛЮЦИЯ): глобальный срез 35% HP всех мобов и тотальный стазис
                WeaponType::InfiniteDilation => {
                    let combat_radius = 420.0;
                    let nearby_indices = world.spatial_grid.query_nearby(boss_pos, combat_radius);
                    for idx in nearby_indices {
                        if let Some(mob) = world.mobs.get_mut(idx) {
                            if mob.health.is_alive() && mob.pos.distance_squared(boss_pos) < combat_radius * combat_radius {
                                let cut_damage = (mob.health.current * 0.35).max(effective_damage);
                                mob.status.apply_freeze(4.0);
                                world.damage_queue.push(DamagePacket::mob(
                                    Some(0),
                                    idx,
                                    mob.id,
                                    cut_damage,
                                    DamageType::True,
                                    1.0,
                                ).with_owner(owner_boss_id));
                            }
                        }
                    }
                    world.spawn_particles(boss_pos, 70, SKYBLUE, combat_radius);
                }

                // 1.55 Астральный Саван: генерация защитных барьеров
                WeaponType::AstralCloak => {
                    if world.boss_inventory.astral_cloak_charges < 3 {
                        world.boss_inventory.astral_cloak_charges += 1;
                        world.spawn_particles(boss_pos, 25, BLANK, 60.0);
                    }
                }

                // 1.56 Саван Кровавого Императора (ЭВОЛЮЦИЯ): до 4 барьеров и усиленная защита
                WeaponType::BloodEmperorShroud => {
                    if world.boss_inventory.astral_cloak_charges < 4 {
                        world.boss_inventory.astral_cloak_charges += 1;
                        world.spawn_particles(boss_pos, 35, MAROON, 80.0);
                    }
                }
            }
        }
    }

    world.boss_inventory.weapons = weapons;

    // Пассивная регенерация здоровья от CrimsonHeartstone и др.
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

        let beam_len = 450.0;
        let beam_end = boss_pos + target_dir * beam_len;
        world.death_ray_active = Some((boss_pos, beam_end, 0.28));

        let ray_norm = target_dir;
        if !world.spatial_grid.is_empty() && world.spatial_grid.len() == world.mobs.len() {
            let spatial_grid = &world.spatial_grid;
            let mobs = &world.mobs;
            let damage_queue = &mut world.damage_queue;
            spatial_grid.query_segment_thick(boss_pos, beam_end, 188.0, |mob_idx| {
                if let Some(mob) = mobs.get(mob_idx) {
                    if !mob.health.is_alive() {
                        return;
                    }
                    let to_mob = mob.pos - boss_pos;
                    let projection = to_mob.dot(ray_norm);
                    if projection > 0.0 && projection < beam_len {
                        let closest_pt = boss_pos + ray_norm * projection;
                        if mob.pos.distance_squared(closest_pt) < (mob.radius + 28.0).powi(2) {
                            damage_queue.push(DamagePacket::mob(
                                Some(0),
                                mob_idx,
                                mob.id,
                                death_ray_damage,
                                DamageType::True,
                                1.0,
                            ).with_owner(owner_boss_id));
                        }
                    }
                }
            });
        } else {
            for (mob_idx, mob) in world.mobs.iter().enumerate() {
                if !mob.health.is_alive() {
                    continue;
                }
                let to_mob = mob.pos - boss_pos;
                let projection = to_mob.dot(ray_norm);
                if projection > 0.0 && projection < beam_len {
                    let closest_pt = boss_pos + ray_norm * projection;
                    if mob.pos.distance_squared(closest_pt) < (mob.radius + 28.0).powi(2) {
                        world.damage_queue.push(DamagePacket::mob(
                            Some(0),
                            mob_idx,
                            mob.id,
                            death_ray_damage,
                            DamageType::True,
                            1.0,
                        ).with_owner(owner_boss_id));
                    }
                }
            }
        }

        // Луч Смерти прожигает и Алтари Крови
        for st in &world.structures {
            let to_st = st.pos - boss_pos;
            let projection = to_st.dot(ray_norm);
            if projection > 0.0 && projection < beam_len {
                let closest_pt = boss_pos + ray_norm * projection;
                if st.pos.distance_squared(closest_pt) < (st.radius + 30.0).powi(2) {
                    world.damage_queue.push(DamagePacket::structure(
                        Some(0),
                        st.id,
                        death_ray_damage * 1.5,
                        DamageType::True,
                        1.0,
                    ).with_owner(owner_boss_id));
                }
            }
        }
    }

    // Модификаторы снарядов от пассивок и реликвий
    let has_colossal_eye = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::ColossalEye);
    let has_elder_mage_shard = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::ElderMageShard);
    let has_solar_relic = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::SolarRelic);

    // Аура Солнечного Венца (SolarRelic): наносит 40 святого урона/сек вокруг босса
    if has_solar_relic {
        let solar_rad = 180.0;
        let nearby = world.spatial_grid.query_nearby(boss_pos, solar_rad);
        for idx in nearby {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(boss_pos) < solar_rad * solar_rad {
                    world.damage_queue.push(DamagePacket::mob(
                        Some(0),
                        idx,
                        mob.id,
                        40.0 * dt,
                        DamageType::Holy,
                        0.3,
                    ).with_owner(owner_boss_id));
                }
            }
        }
    }

    // Присваиваем ID и применяем эффекты реликвий и автонаведения
    let has_blessed_heart = world.boss_inventory.has_passive(crate::ecs::components::inventory::PassiveType::BlessedHeart);

    for proj in &mut new_projectiles {
        proj.id = world.alloc_id();
        proj.prev_pos = proj.pos;
        if proj.faction == Faction::Boss {
            proj.owner_boss_id = owner_boss_id;
        }

        // 1. Врождённое самонаведение специфических типов оружия
        match proj.source_weapon {
            Some(WeaponType::MissileSwarm) => proj.homing_strength = 7.5,
            Some(WeaponType::DoomsdayBarrage) => proj.homing_strength = 9.5,
            Some(WeaponType::SoulPhantoms) => proj.homing_strength = 6.0,
            Some(WeaponType::BansheeWail) => proj.homing_strength = 8.0,
            Some(WeaponType::ArcaneScepter) => proj.homing_strength = 4.5,
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
        if has_elder_mage_shard {
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
pub fn update_enemy_mob_attacks(world: &mut World, _dt: f32) {
    let primary_pos = world.boss_pos;
    let primary_vel = world.boss_vel;
    let primary_alive = world.boss_health.is_alive();
    let coop_bosses = &world.coop_bosses;
    let mut enemy_shots = Vec::new();

    for mob in &mut world.mobs {
        if !mob.health.is_alive() { continue; }
        let assigned_target = mob.target_boss_id.and_then(|id| {
            if id == 1 && primary_alive {
                Some((primary_pos, primary_vel))
            } else {
                coop_bosses.iter()
                    .find(|player| player.id == id && player.boss_health.is_alive())
                    .map(|player| (player.boss_pos, player.boss_vel))
            }
        });
        let mut nearest_target = if primary_alive {
            Some((primary_pos.distance_squared(mob.pos), primary_pos, primary_vel))
        } else { None };
        for player in coop_bosses.iter().filter(|player| player.boss_health.is_alive()) {
            let distance = player.boss_pos.distance_squared(mob.pos);
            if nearest_target.as_ref().map_or(true, |(best, _, _)| distance < *best) {
                nearest_target = Some((distance, player.boss_pos, player.boss_vel));
            }
        }
        let Some((target_pos, target_vel)) = assigned_target.or_else(|| nearest_target.map(|(_, pos, vel)| (pos, vel))) else {
            continue;
        };
        let dist_sq = mob.pos.distance_squared(target_pos);
        let shot = if mob.tier == 3 || mob.tier == 11 {
            let range = if mob.tier == 11 { 380.0 } else { 310.0 };
            if dist_sq < range * range && mob.attack_timer <= 0.0 {
                mob.attack_timer = if mob.tier == 11 { 1.6 } else { 2.0 };
                let (damage, damage_type, speed) = if mob.tier == 11 {
                    (48.0, DamageType::True, 260.0)
                } else {
                    (18.0, DamageType::Physical, 230.0)
                };
                Some((mob.pos, target_pos, target_vel, damage, damage_type, speed, 9.0, None))
            } else { None }
        } else if mob.tier == 7 || mob.tier == 15 {
            let range = if mob.tier == 15 { 440.0 } else { 360.0 };
            if dist_sq < range * range && mob.attack_timer <= 0.0 {
                mob.attack_timer = if mob.tier == 15 { 1.8 } else { 2.2 };
                let (damage, speed) = if mob.tier == 15 { (110.0, 300.0) } else { (35.0, 260.0) };
                Some((mob.pos, target_pos, target_vel, damage, DamageType::Fire, speed, 14.0, None))
            } else { None }
        } else if mob.tier == 8 || mob.tier == 16 {
            let range = if mob.tier == 16 { 500.0 } else { 420.0 };
            if dist_sq < range * range && mob.attack_timer <= 0.0 {
                mob.attack_timer = if mob.tier == 16 { 2.4 } else { 2.8 };
                let (damage, speed) = if mob.tier == 16 { (160.0, 320.0) } else { (70.0, 280.0) };
                Some((mob.pos, target_pos, target_vel, damage, DamageType::True, speed, 20.0, Some(WeaponType::ExecutionerAxe)))
            } else { None }
        } else { None };

        if let Some(shot) = shot { enemy_shots.push(shot); }
    }

    for (pos, target_pos, target_vel, damage, damage_type, speed, radius, source_weapon) in enemy_shots {
        let id = world.alloc_id();
        let direction = crate::core::math::calculate_lead_direction(pos, target_pos, target_vel, speed)
            .unwrap_or_else(|| (target_pos - pos).normalize_or_zero());
        world.projectiles.push(ProjectileEntity {
            id,
            pos,
            prev_pos: pos,
            vel: direction * speed,
            damage,
            damage_type,
            armor_penetration: 0.20,
            lifetime: 2.5,
            radius,
            pierce: 1,
            faction: Faction::Director,
            owner_boss_id: None,
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

    // 3.1 Цикл активации Угловых Инфернальных Разломов (раз в 60 секунд после закрытия/прорыва предыдущего)
    let any_obelisk_active = world.structures.iter().any(|s| {
        s.structure_type == crate::content::structures::STRUCTURE_OBELISK && s.is_active
    });
    let hero_alive = world.mobs.iter().any(|m| m.is_hero && m.health.is_alive());

    if !any_obelisk_active && !hero_alive {
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
                let obelisk_hp = crate::content::structures::OBELISK_BASE_HP_PER_BOSS * world.living_boss_count().max(1) as f32;
                let choice = rand::gen_range(0, obelisk_indices.len());
                let target_idx = obelisk_indices[choice];
                if let Some(obelisk) = world.structures.get_mut(target_idx) {
                    obelisk.is_active = true;
                    obelisk.state_timer = crate::content::structures::OBELISK_CHARGE_SECONDS;
                    obelisk.health.max = obelisk_hp;
                    obelisk.health.current = obelisk_hp;
                    let pos = obelisk.pos;

                    let half_w = crate::core::MAP_WIDTH * 0.5;
                    let half_h = crate::core::MAP_HEIGHT * 0.5;
                    let corner_name = if pos.x < half_w && pos.y < half_h {
                        "Северо-Запад"
                    } else if pos.x >= half_w && pos.y < half_h {
                        "Северо-Восток"
                    } else if pos.x < half_w && pos.y >= half_h {
                        "Юго-Запад"
                    } else {
                        "Юго-Восток"
                    };

                    world.screen_shake = 22.0;
                    world.push_audio(crate::audio::AudioEvent::ObeliskActivate);

                    if !world.is_headless {
                        // Плавающий текст на самом обелиске
                        world.add_floating_text(
                            pos,
                            format!("ИНФЕРНАЛЬНЫЙ РАЗЛОМ [{}]! 3 МИНУТЫ ДО ПРОРЫВА!", corner_name),
                            GOLD,
                            4.0,
                        );

                        // Уведомление непосредственно над Боссом (и союзными боссами),
                        // чтобы игрок сразу узнал об активации даже вдали от обелиска!
                        if world.boss_health.is_alive() {
                            world.add_floating_text(
                                world.boss_pos,
                                format!("⚠️ ТРЕЩИТ ЗЕМЛЯ! АКТИВИРОВАН РАЗЛОМ: [{}]!", corner_name),
                                GOLD,
                                4.5,
                            );
                        }
                        let coop_notify_positions: Vec<Vec2> = world
                            .coop_bosses
                            .iter()
                            .filter(|p| p.boss_health.is_alive())
                            .map(|p| p.boss_pos)
                            .collect();
                        for player_pos in coop_notify_positions {
                            world.add_floating_text(
                                player_pos,
                                format!("⚠️ ТРЕЩИТ ЗЕМЛЯ! АКТИВИРОВАН РАЗЛОМ: [{}]!", corner_name),
                                GOLD,
                                4.5,
                            );
                        }
                    }

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
    }

    // 3.2 Таймер зарядки активного разлома и Прорыв
    let obelisk_reset_hp = crate::content::structures::OBELISK_BASE_HP_PER_BOSS * world.living_boss_count().max(1) as f32;
    let mut breach_event = None;
    for st in &mut world.structures {
        if st.structure_type == crate::content::structures::STRUCTURE_OBELISK && st.is_active {
            st.state_timer -= dt;
            if st.state_timer <= 0.0 {
                st.is_active = false;
                st.health.max = obelisk_reset_hp;
                st.health.current = st.health.max;
                breach_event = Some((st.id, st.pos));
            }
        }
    }

    // ИНФЕРНАЛЬНЫЙ ПРОРЫВ: призыв элитного легиона по формуле (доход игрока * 15)
    if let Some((st_id, pos)) = breach_event {
        world.structure_contributors.remove(&st_id);
        world.obelisk_contributors.remove(&st_id);
        world.screen_shake = 22.0;
        world.spawn_particles(pos, 50, PURPLE, 240.0);
        world.spawn_particles(pos, 35, RED, 180.0);
        world.add_floating_text(pos, "ИНФЕРНАЛЬНЫЙ ПРОРЫВ! ОРДА ВЫРВАЛАСЬ!".to_string(), RED, 2.5);

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

/// Обновление чёрных дыр: 1 сек красное предупреждение, затем 10 сек вращение вокруг хозяина (2 полных оборота),
/// притягивание, стан и периодический нефизический урон боссам.
pub fn update_black_holes(world: &mut World, dt: f32) {
    if world.black_holes.is_empty() {
        return;
    }

    let mut hole_damage_events = Vec::new();

    for bh in world.black_holes.iter_mut() {
        bh.duration -= dt;
        bh.spin_angle += 6.0 * dt;

        // Первую секунду (duration > 10.0) — предупреждение (is_active = false)
        if bh.duration > 10.0 {
            bh.is_active = false;
            continue;
        }
        bh.is_active = true;

        // Вращение по орбите вокруг хозяина
        let mut owner_pos = bh.center_pos;
        if let Some(owner) = world.mobs.iter().find(|m| m.id == bh.owner_id && m.health.is_alive()) {
            owner_pos = owner.pos;
        }

        bh.orbit_angle += bh.orbit_speed * dt;
        bh.center_pos = owner_pos + Vec2::new(bh.orbit_angle.cos(), bh.orbit_angle.sin()) * bh.orbit_dist;

        let pull_rad = bh.radius * 2.5;
        hole_damage_events.push((bh.center_pos, bh.radius, pull_rad));
    }

    world.black_holes.retain(|bh| bh.duration > 0.0);

    for (center, radius, pull_rad) in hole_damage_events {
        let pull_r2 = pull_rad * pull_rad;
        let rad_r2 = radius * radius;

        // Основной босс
        if world.boss_health.is_alive() {
            let d2 = world.boss_pos.distance_squared(center);
            if d2 < pull_r2 {
                let dist = d2.sqrt().max(0.1);
                let dir = (center - world.boss_pos) / dist;
                world.boss_pos += dir * (140.0 * dt);
                if d2 < rad_r2 {
                    let tangent = Vec2::new(-dir.y, dir.x);
                    world.boss_pos += tangent * (90.0 * dt);
                    world.boss_health.take_damage(85.0 * dt);
                    world.boss_damage_taken_timer = 3.0;
                    world.boss_still_time = 0.0;
                }
            }
        }

        // Кооп боссы
        for coop in world.coop_bosses.iter_mut() {
            if coop.boss_health.is_alive() {
                let d2 = coop.boss_pos.distance_squared(center);
                if d2 < pull_r2 {
                    let dist = d2.sqrt().max(0.1);
                    let dir = (center - coop.boss_pos) / dist;
                    coop.boss_pos += dir * (140.0 * dt);
                    if d2 < rad_r2 {
                        let tangent = Vec2::new(-dir.y, dir.x);
                        coop.boss_pos += tangent * (90.0 * dt);
                        coop.boss_health.take_damage(85.0 * dt);
                        coop.boss_damage_taken_timer = 3.0;
                        coop.boss_still_time = 0.0;
                    }
                }
            }
        }
    }
}

/// Обновление Героев Директора (Tier 17 Пустота, Tier 18 Инферно) и их копий:
/// Таймер ярости (через 60с: +1% скорости и +1% размера в секунду), уникальные способности и спавн клонов.
pub fn update_heroes_and_clones(world: &mut World, dt: f32) {
    // 1. Кулдаун призыва героев у Директоров
    for dir in world.directors.iter_mut() {
        if dir.hero_cooldown > 0.0 {
            dir.hero_cooldown = (dir.hero_cooldown - dt).max(0.0);
        }
    }

    // Собираем позиции живых боссов без удержания заимствования world
    let mut live_bosses = Vec::new();
    if world.boss_health.is_alive() {
        live_bosses.push(world.boss_pos);
    }
    for coop in &world.coop_bosses {
        if coop.boss_health.is_alive() {
            live_bosses.push(coop.boss_pos);
        }
    }

    let find_nearest_boss = |from: Vec2| -> Vec2 {
        let mut best = from;
        let mut min_d2 = f32::MAX;
        for &bp in &live_bosses {
            let d2 = from.distance_squared(bp);
            if d2 < min_d2 {
                min_d2 = d2;
                best = bp;
            }
        }
        best
    };

    struct BlackHoleSpawn {
        owner_id: usize,
        target_pos: Vec2,
        orbit_dist: f32,
        init_angle: f32,
    }

    struct ProjSpawn {
        pos: Vec2,
        vel: Vec2,
        damage: f32,
        damage_type: DamageType,
        armor_pen: f32,
        pierce: u8,
        radius: f32,
        lifetime: f32,
    }

    let mut hole_requests = Vec::new();
    let mut clone_requests = Vec::new();
    let mut proj_requests = Vec::new();
    let mut cage_requests = Vec::new();
    let mut absorb_gem_requests = Vec::new();
    let mut hero_heals = Vec::new();

    for (i, mob) in world.mobs.iter_mut().enumerate() {
        if !mob.health.is_alive() {
            continue;
        }

        // 2. Логика Героев (is_hero)
        if mob.is_hero {
            // Обратный отсчёт до ярости (60с)
            mob.hero_enrage_timer -= dt;
            if mob.hero_enrage_timer <= 0.0 {
                // Режим ярости: +1% к скорости и +1% к размеру модельки и хитбокса каждую секунду
                mob.speed *= 1.0 + 0.01 * dt;
                mob.radius = (mob.radius * (1.0 + 0.01 * dt)).min(160.0);
            }

            // Откат 4 способностей
            for timer in mob.hero_skill_timers.iter_mut() {
                if *timer > 0.0 {
                    *timer = (*timer - dt).max(0.0);
                }
            }

            let hero_pos = mob.pos;
            let hero_id = mob.id;
            let hero_tier = mob.tier;

            let target_pos = find_nearest_boss(hero_pos);
            let dist_to_target = hero_pos.distance(target_pos);

            if hero_tier == 17 {
                // --- ГЕРОЙ 1: ПУСТОТА ---
                // Скилл 0: Чёрная Дыра (КД 35с). Подходит к боссу на расстояние [дальность каста 280] - [диаметр 140] = 140 px
                if mob.hero_skill_timers[0] <= 0.0 && dist_to_target <= 320.0 {
                    mob.hero_skill_timers[0] = 35.0;
                    let orbit_dist = dist_to_target.clamp(120.0, 280.0);
                    let init_angle = (target_pos.y - hero_pos.y).atan2(target_pos.x - hero_pos.x);
                    hole_requests.push(BlackHoleSpawn {
                        owner_id: hero_id,
                        target_pos,
                        orbit_dist,
                        init_angle,
                    });
                }

                // Скилл 1: Копии Пустоты (КД 24с). Призывает 2 мелких стреляющих копии
                if mob.hero_skill_timers[1] <= 0.0 {
                    mob.hero_skill_timers[1] = 24.0;
                    for k in 0..2 {
                        let offset = Vec2::new(
                            if k == 0 { -30.0 } else { 30.0 },
                            rand::gen_range(-20.0, 20.0),
                        );
                        clone_requests.push((hero_pos + offset, hero_id));
                    }
                }

                // Скилл 2: Гравитационный Коллапс (КД 20с). Поглощает кристаллы душ в 500px, получая щит/исцеление
                if mob.hero_skill_timers[2] <= 0.0 {
                    mob.hero_skill_timers[2] = 20.0;
                    absorb_gem_requests.push((i, hero_pos));
                }

                // Скилл 3: Астральная Сингулярность (КД 16с). 8 спиральных сфер Бездны, замедляющих боссов
                if mob.hero_skill_timers[3] <= 0.0 {
                    mob.hero_skill_timers[3] = 16.0;
                    for k in 0..8 {
                        let angle = (k as f32) * (std::f32::consts::PI / 4.0);
                        let dir = Vec2::new(angle.cos(), angle.sin());
                        proj_requests.push(ProjSpawn {
                            pos: hero_pos,
                            vel: dir * 160.0,
                            damage: 28.0,
                            damage_type: DamageType::True,
                            armor_pen: 0.3,
                            pierce: 3,
                            radius: 12.0,
                            lifetime: 3.5,
                        });
                    }
                }
            } else if hero_tier == 18 {
                // --- ГЕРОЙ 2: АРХИДЕМОН ИНФЕРНО ---
                // Скилл 0: Извержение Магмы (КД 22с)
                if mob.hero_skill_timers[0] <= 0.0 {
                    mob.hero_skill_timers[0] = 22.0;
                    for _ in 0..3 {
                        let offset = Vec2::new(rand::gen_range(-120.0, 120.0), rand::gen_range(-120.0, 120.0));
                        proj_requests.push(ProjSpawn {
                            pos: target_pos + offset,
                            vel: Vec2::ZERO,
                            damage: 65.0,
                            damage_type: DamageType::Fire,
                            armor_pen: 0.2,
                            pierce: 99,
                            radius: 36.0,
                            lifetime: 1.5,
                        });
                    }
                }

                // Скилл 1: Инфернальная Клетка (КД 30с). Запирает босса в кольцо костяных стен
                if mob.hero_skill_timers[1] <= 0.0 && dist_to_target <= 450.0 {
                    mob.hero_skill_timers[1] = 30.0;
                    cage_requests.push(target_pos);
                }

                // Скилл 2: Пылающий Панцирь (КД 18с). Пульсирует огнем в радиусе 160px
                if mob.hero_skill_timers[2] <= 0.0 {
                    mob.hero_skill_timers[2] = 18.0;
                    mob.status.burn_timer = 6.0;
                }

                // Скилл 3: Метеор Апокалипсиса (КД 40с)
                if mob.hero_skill_timers[3] <= 0.0 {
                    mob.hero_skill_timers[3] = 40.0;
                    proj_requests.push(ProjSpawn {
                        pos: target_pos,
                        vel: Vec2::new(0.0, 60.0),
                        damage: 160.0,
                        damage_type: DamageType::Fire,
                        armor_pen: 0.4,
                        pierce: 99,
                        radius: 55.0,
                        lifetime: 2.2,
                    });
                }
            }
        }

        // 3. Логика мелких копий Пустоты (Void Clones)
        if !mob.is_hero && mob.tier == 17 && mob.owner_hero_id.is_some() {
            mob.hero_skill_timers[0] -= dt;
            if mob.hero_skill_timers[0] <= 0.0 {
                mob.hero_skill_timers[0] = 2.4;
                mob.attack_count += 1;

                let target = find_nearest_boss(mob.pos);
                let dir = (target - mob.pos).normalize_or_zero();
                proj_requests.push(ProjSpawn {
                    pos: mob.pos,
                    vel: dir * 190.0,
                    damage: 18.0,
                    damage_type: DamageType::True,
                    armor_pen: 0.2,
                    pierce: 1,
                    radius: 7.0,
                    lifetime: 3.0,
                });

                // Каждая атака мелкой копии лечит хозяина-Пустоту!
                if let Some(owner_id) = mob.owner_hero_id {
                    hero_heals.push((owner_id, 18.0));
                }

                // Сделав 6 атак, они дублируются один раз и полностью исцеляются
                if mob.attack_count >= 6 && !mob.has_duplicated {
                    mob.has_duplicated = true;
                    mob.health.current = mob.health.max;
                    clone_requests.push((mob.pos + Vec2::new(rand::gen_range(-18.0, 18.0), rand::gen_range(-18.0, 18.0)), mob.owner_hero_id.unwrap_or(0)));
                }
            }
        }
    }

    // Обработка поглощения душ
    for (mob_idx, hero_pos) in absorb_gem_requests {
        let mut absorbed = 0;
        world.gems.retain(|gem| {
            if gem.pos.distance_squared(hero_pos) < 500.0 * 500.0 && absorbed < 8 {
                absorbed += 1;
                false
            } else {
                true
            }
        });
        if absorbed > 0 {
            if let Some(mob) = world.mobs.get_mut(mob_idx) {
                mob.health.heal(absorbed as f32 * 45.0);
            }
        }
    }

    // Создание Чёрных дыр
    for bh in hole_requests {
        let id = world.alloc_id();
        world.black_holes.push(crate::ecs::world::BlackHoleEntity {
            id,
            owner_id: bh.owner_id,
            center_pos: bh.target_pos,
            radius: 70.0,
            orbit_angle: bh.init_angle,
            orbit_dist: bh.orbit_dist,
            orbit_speed: 4.0 * std::f32::consts::PI / 10.0,
            duration: 11.0,
            is_active: false,
            spin_angle: 0.0,
        });
    }

    // Создание снарядов
    for p in proj_requests {
        let id = world.alloc_id();
        world.projectiles.push(ProjectileEntity {
            id,
            pos: p.pos,
            prev_pos: p.pos,
            vel: p.vel,
            damage: p.damage,
            damage_type: p.damage_type,
            armor_penetration: p.armor_pen,
            pierce: p.pierce,
            radius: p.radius,
            lifetime: p.lifetime,
            faction: Faction::Director,
            owner_boss_id: None,
            source_weapon: None,
            homing_strength: 0.0,
        });
    }

    // Создание клонов
    for (clone_pos, owner_id) in clone_requests {
        let id = world.alloc_id();
        let carapace_level = world.directors.first().map_or(0, |d| d.grade_carapace);
        let mut resistances = crate::ecs::components::combat::Resistances::default();
        crate::content::structures::apply_carapace(&mut resistances, carapace_level);
        world.mobs.push(crate::ecs::world::MobEntity {
            id,
            target_boss_id: None,
            pos: clone_pos,
            vel: Vec2::ZERO,
            speed: 65.0,
            health: crate::ecs::components::combat::Health::new(490.0),
            resistances,
            tier: 17,
            radius: 15.0,
            damage: 16.0,
            status: crate::ecs::components::combat::StatusEffects::default(),
            is_undead: false,
            is_elite: false,
            knockback_immune: false,
            knockback_vel: Vec2::ZERO,
            attack_timer: 0.0,
            is_hero: false,
            hero_enrage_timer: 999.0,
            hero_skill_timers: [0.0; 4],
            attack_count: 0,
            has_duplicated: true, // новые дубликаты уже не дублируются снова
            owner_hero_id: Some(owner_id),
            last_hit_boss_id: None,
        });
    }

    // Создание стен клетки Инферно
    for cage_pos in cage_requests {
        for k in 0..6 {
            let angle = (k as f32) * (std::f32::consts::PI / 3.0);
            let wall_pos = cage_pos + Vec2::new(angle.cos(), angle.sin()) * 110.0;
            let id = world.alloc_id();
            let carapace_level = world.directors.first().map_or(0, |d| d.grade_carapace);
            let mut resistances = crate::ecs::components::combat::Resistances::default();
            crate::content::structures::apply_carapace(&mut resistances, carapace_level);
            world.structures.push(crate::ecs::world::StructureEntity {
                id,
                pos: wall_pos,
                health: crate::ecs::components::combat::Health::new(800.0),
                resistances,
                faction: Faction::Director,
                radius: 20.0,
                structure_type: crate::content::structures::STRUCTURE_WALL,
                pulse_timer: 0.0,
                spawn_timer: 0.0,
                lifetime: 6.0,
                max_lifetime: 6.0,
                is_active: true,
                state_timer: 0.0,
            });
        }
    }

    // Применение исцеления героям
    for (hero_id, amount) in hero_heals {
        if let Some(hero) = world.mobs.iter_mut().find(|m| m.id == hero_id && m.health.is_alive()) {
            hero.health.heal(amount);
        }
    }
}

