use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::{DamageSourceKind, PassiveType, WeaponType};
use crate::ecs::world::{SoulGemEntity, World};

fn apply_damage_to_boss(
    world: &mut World,
    mut amount: f32,
    damage_type: DamageType,
    source_kind: DamageSourceKind,
) {
    if amount <= 0.0 {
        return;
    }

    // 1. Проверка защитных зарядов Астрального Савана (AstralCloak) и Савана Кровавого Императора (BloodEmperorShroud)
    let has_shroud = world.boss_inventory.has_weapon(WeaponType::BloodEmperorShroud);
    if world.boss_inventory.astral_cloak_charges > 0 {
        world.boss_inventory.astral_cloak_charges -= 1;
        world.add_floating_text(world.boss_pos, "БАРЬЕР".to_string(), SKYBLUE, 1.0);

        if has_shroud {
            // Ответная волна крови при срабатывании барьера
            let boss_pos = world.boss_pos;
            world.spawn_particles(boss_pos, 35, RED, 220.0);
            for mob in &mut world.mobs {
                if mob.pos.distance_squared(boss_pos) < 220.0 * 220.0 {
                    mob.health.take_damage(200.0);
                }
            }
        }
        return;
    }

    // 2. Саван Кровавого Императора: ограничение любого входящего удара максимумом в 12 урона + контратака
    if has_shroud {
        amount = amount.min(12.0);
        let boss_pos = world.boss_pos;
        world.spawn_particles(boss_pos, 15, MAROON, 150.0);
        for mob in &mut world.mobs {
            if mob.pos.distance_squared(boss_pos) < 150.0 * 150.0 {
                mob.health.take_damage(75.0);
            }
        }
    }

    world.boss_health.take_damage(amount);
    world.boss_inventory.record_damage_taken(amount, damage_type, source_kind);
    world.total_damage_dealt_to_boss += amount;
    world.boss_damage_taken_timer = 3.0;
    world.boss_still_time = 0.0;
}

pub fn update_combat(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let boss_radius = 24.0;
    let is_boss_invulnerable = world.boss_invulnerable_timer > 0.0;

    let has_blade_storm = world.boss_inventory.has_weapon(WeaponType::BladeStorm);
    let has_holy_hammer = world.boss_inventory.has_weapon(WeaponType::HolyHammer);
    let has_divine_shield = world.boss_inventory.has_weapon(WeaponType::DivineShield);
    let has_hellfire = world.boss_inventory.has_weapon(WeaponType::Hellfire);
    let has_aegis_orbiters = world.boss_inventory.has_weapon(WeaponType::AegisOrbiters);
    let has_invincible_bastion = world.boss_inventory.has_weapon(WeaponType::InvincibleBastion);
    let has_sacred_scriptures = world.boss_inventory.has_weapon(WeaponType::SacredScriptures);
    let has_infernal_liturgy = world.boss_inventory.has_weapon(WeaponType::InfernalLiturgy);
    let has_octarine_core = world.boss_inventory.has_passive(PassiveType::OctarineCore);
    let has_pyromaniac = world.boss_inventory.has_passive(PassiveType::Pyromaniac);
    let has_immortal_aegis = world.boss_inventory.has_passive(PassiveType::ImmortalAegis);
    let has_soul_feast = world.boss_inventory.has_weapon(WeaponType::SoulFeast);
    let has_static_core = world.boss_inventory.has_passive(PassiveType::StaticCore);
    let has_ukulele = world.boss_inventory.has_passive(PassiveType::UkuleleResonance);
    let has_siege_mortar = world.boss_inventory.has_passive(PassiveType::SiegeMortar);
    let has_soul_harvester = world.boss_inventory.has_passive(PassiveType::SoulHarvester);
    let tesla_chance = world.boss_inventory.get_tesla_chain_chance();
    let crit_chance = world.boss_inventory.get_crit_chance();
    let dmg_mult = world.boss_inventory.get_damage_multiplier();
    let curse_xp_mult = world.boss_inventory.get_curse_stone_xp_multiplier();

    let mut kamikaze_explosions = Vec::new();
    let mut boss_contact_hits = Vec::new();
    // Глобальный переиспользуемый буфер для всех spatial_grid запросов функции update_combat
    let mut nearby_scratch: Vec<usize> = Vec::with_capacity(128);

    // 1. Коллизии мобов с Боссом и контактный урон
    let mut contact_kamikaze_blast = false;
    world.spatial_grid.query_nearby_into(boss_pos, 120.0, &mut nearby_scratch);
    for &idx in &nearby_scratch {
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
                    boss_contact_hits.push((actual_dmg, packet.damage_type));

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
                    amount: 65.0 * dt * dmg_mult,
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

            // 1.2 Святой Молот (HolyHammer) — вращающиеся освященные молоты
            if has_holy_hammer && dist_sq < 88.0 * 88.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 60.0 * dt * dmg_mult,
                    damage_type: DamageType::Holy,
                    armor_penetration: 0.25,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);
            }

            // 1.3 Эволюция: Праведный Щит (DivineShield) — щит с вампиризмом
            if has_divine_shield && dist_sq < 98.0 * 98.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 80.0 * dt * dmg_mult,
                    damage_type: DamageType::Holy,
                    armor_penetration: 0.20,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);

                // Вампиризм: 5% урона возвращается в здоровье босса
                world.boss_health.heal(dmg * 0.05);
            }

            // 1.4 Орбитальные Бастионы (AegisOrbiters) — защитные барьеры
            if has_aegis_orbiters && dist_sq < 106.0 * 106.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 70.0 * dt * dmg_mult,
                    damage_type: DamageType::Physical,
                    armor_penetration: 0.35,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);
            }

            // 1.5 Эволюция: Несокрушимая Крепость (InvincibleBastion) — сверхпрочное силовое поле
            if has_invincible_bastion && dist_sq < 125.0 * 125.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 115.0 * dt * dmg_mult,
                    damage_type: DamageType::Holy,
                    armor_penetration: 0.50,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);
            }

            // 1.6 Орбитальный Гримуар (SacredScriptures) — священные вращающиеся фолианты
            if has_sacred_scriptures && dist_sq < 92.0 * 92.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 55.0 * dt * dmg_mult,
                    damage_type: DamageType::Holy,
                    armor_penetration: 0.30,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);
            }

            // 1.7 Эволюция: Инфернальная Литургия (InfernalLiturgy) — адские пылающие скрижали
            if has_infernal_liturgy && dist_sq < 115.0 * 115.0 {
                let packet = DamagePacket {
                    source_id: Some(0),
                    target_id: mob.id,
                    amount: 110.0 * dt * dmg_mult,
                    damage_type: DamageType::Fire,
                    armor_penetration: 0.50,
                };
                let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(dmg);
            }
        }
    }

    for (actual_dmg, dmg_type) in boss_contact_hits {
        apply_damage_to_boss(world, actual_dmg, dmg_type, DamageSourceKind::Melee);
    }

    if contact_kamikaze_blast {
        world.push_audio(crate::audio::AudioEvent::FireBlast);
    }
    // Звук урона по боссу отключен для чистоты аудиомикса

    // 1.3 Контактный урон аур по разрушаемым Алтарям Крови и Обелискам
    let is_reward_structure = |st_type: u8| crate::content::structures::is_altar_structure_type(st_type) || st_type == crate::content::structures::STRUCTURE_OBELISK;
    let mut structure_contribution_ids = Vec::new();
    for st in &mut world.structures {
        if !crate::content::structures::boss_can_damage_structure(st) {
            continue;
        }
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
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
        }

        if has_holy_hammer && dist_sq < (88.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 70.0 * dt,
                damage_type: DamageType::Holy,
                armor_penetration: 0.25,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
        }

        if has_divine_shield && dist_sq < (98.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 90.0 * dt,
                damage_type: DamageType::Holy,
                armor_penetration: 0.20,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
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
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
        }

        if has_aegis_orbiters && dist_sq < (106.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 80.0 * dt,
                damage_type: DamageType::Physical,
                armor_penetration: 0.35,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
        }

        if has_invincible_bastion && dist_sq < (125.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 110.0 * dt,
                damage_type: DamageType::Holy,
                armor_penetration: 0.50,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
        }

        if has_sacred_scriptures && dist_sq < (92.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 65.0 * dt,
                damage_type: DamageType::Holy,
                armor_penetration: 0.30,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
        }

        if has_infernal_liturgy && dist_sq < (115.0 + st.radius).powi(2) {
            let packet = DamagePacket {
                source_id: Some(0),
                target_id: st.id,
                amount: 120.0 * dt,
                damage_type: DamageType::Fire,
                armor_penetration: 0.50,
            };
            let dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(dmg);
            if is_reward_structure(st.structure_type) && dmg > 0.0 { structure_contribution_ids.push(st.id); }
        }
    }
    for id in structure_contribution_ids.drain(..) { world.record_structure_contribution(id); }

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
        world.spatial_grid.query_nearby_into(exp_pos, exp_radius, &mut nearby_scratch);
        for &idx in &nearby_scratch {
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
    let mut queued_obelisk_contributions = Vec::new();

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
                let source_kind = if packet.source_id.is_none() {
                    DamageSourceKind::Hazard
                } else {
                    DamageSourceKind::Melee
                };
                apply_damage_to_boss(world, actual_dmg, packet.damage_type, source_kind);
            }
        } else if let Some(&mob_idx) = mob_index.get(&packet.target_id) {
            if let Some(mob) = world.mobs.get_mut(mob_idx) {
                let actual_dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                mob.health.take_damage(actual_dmg);
            }
        } else if let Some(st) = world
            .structures
            .iter_mut()
            .find(|s| s.id == packet.target_id && crate::content::structures::boss_can_damage_structure(s))
        {
            let actual_dmg = st.resistances.calculate_damage(&packet, false);
            st.health.take_damage(actual_dmg);
            if is_reward_structure(st.structure_type) && actual_dmg > 0.0 { queued_obelisk_contributions.push(st.id); }
            structure_texts.push((st.pos, format!("-{:.0}", actual_dmg)));
        }
    }
    for id in queued_obelisk_contributions { world.record_structure_contribution(id); }
    for (pos, txt) in structure_texts {
        world.add_floating_text(pos, txt, GOLD, 1.0);
    }

    // 3. Коллизии снарядов
    let mut boss_hits = Vec::new();
    let mut hit_particles = Vec::new();
    let mut crit_texts = Vec::new();
    let mut structure_proj_hits = Vec::new();
    let mut projectile_obelisk_contributions = Vec::new();
    let mut chain_lightning_events: Vec<(Vec2, usize, usize, f32, f32, DamageType)> = Vec::new();
    let mut mortar_explosions: Vec<(Vec2, f32, f32)> = Vec::new();

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
                        let is_crit = crit_chance > 0.0 && world.boss_inventory.roll_luck(crit_chance);
                        let base_damage = if is_crit { proj.damage * 2.0 } else { proj.damage };

                        let packet = DamagePacket {
                            source_id: Some(0),
                            target_id: mob.id,
                            amount: base_damage,
                            damage_type: proj.damage_type,
                            armor_penetration: proj.armor_penetration,
                        };
                        let actual_dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                        mob.health.take_damage(actual_dmg);
                        proj.pierce = proj.pierce.saturating_sub(1);
                        if proj.pierce > 0 {
                            proj.homing_strength = 0.0;
                        }

                        if is_crit {
                            hit_particles.push((mob.pos, GOLD));
                            crit_texts.push((mob.pos, format!("КРИТ {:.0}!", actual_dmg)));
                        }

                        // Цепные молнии от пассивок: Катушка Теслы (TeslaCoil) и Статическое Ядро (StaticCore)
                        if tesla_chance > 0.0 && world.boss_inventory.roll_luck(tesla_chance) {
                            let targets = if has_static_core { 6 } else { 3 };
                            chain_lightning_events.push((mob.pos, mob.id, targets, 230.0, 38.0 * dmg_mult, DamageType::True));
                        }

                        // Резонанс Укулеле (UkuleleResonance): 25% шанс электро-цепочки на 4 цели
                        if has_ukulele && world.boss_inventory.roll_luck(0.25) {
                            chain_lightning_events.push((mob.pos, mob.id, 4, 190.0, 35.0 * dmg_mult, DamageType::True));
                        }

                        // Оружие: Цепная Молния (ChainLightning) — прыгает на 3 цели
                        if proj.source_weapon == Some(WeaponType::ChainLightning) {
                            chain_lightning_events.push((mob.pos, mob.id, 3, 200.0, actual_dmg * 0.75, DamageType::True));
                        }

                        // Оружие (Эволюция): Гнев Бури (StormGodWrath) — сокрушительный разряд на 6 целей
                        if proj.source_weapon == Some(WeaponType::StormGodWrath) {
                            chain_lightning_events.push((mob.pos, mob.id, 6, 260.0, actual_dmg * 1.0, DamageType::True));
                        }

                        // Осадная Мортира (SiegeMortar): взрыв 55px при прямом попадании (50% урона)
                        if has_siege_mortar {
                            mortar_explosions.push((mob.pos, 55.0, proj.damage * 0.5));
                        }

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
                if st.health.is_alive()
                    && crate::content::structures::boss_can_damage_structure(st)
                    && st.pos.distance_squared(proj.pos) < (st.radius + proj.radius).powi(2)
                {
                    let packet = DamagePacket {
                        source_id: Some(0),
                        target_id: st.id,
                        amount: proj.damage,
                        damage_type: proj.damage_type,
                        armor_penetration: proj.armor_penetration,
                    };
                    let actual_dmg = st.resistances.calculate_damage(&packet, false);
                    st.health.take_damage(actual_dmg);
                    if is_reward_structure(st.structure_type) && actual_dmg > 0.0 { projectile_obelisk_contributions.push(st.id); }
                    proj.pierce = proj.pierce.saturating_sub(1);
                    if proj.pierce > 0 {
                        proj.homing_strength = 0.0;
                    }
                    hit_particles.push((st.pos, PURPLE));
                    structure_proj_hits.push((st.pos, format!("-{:.0}", actual_dmg)));
                }
            }
        } else if proj.faction == Faction::Director {
            let dist_to_boss_sq = proj.pos.distance_squared(boss_pos);

            // Парирование / уничтожение вражеских снарядов активными орбиталями и щитами
            let is_parried = (has_blade_storm && dist_to_boss_sq < 85.0 * 85.0)
                || (has_holy_hammer && dist_to_boss_sq < 88.0 * 88.0)
                || (has_divine_shield && dist_to_boss_sq < 98.0 * 98.0)
                || (has_aegis_orbiters && dist_to_boss_sq < 106.0 * 106.0)
                || (has_invincible_bastion && dist_to_boss_sq < 125.0 * 125.0)
                || (has_sacred_scriptures && dist_to_boss_sq < 92.0 * 92.0)
                || (has_infernal_liturgy && dist_to_boss_sq < 115.0 * 115.0)
                || (is_boss_invulnerable && dist_to_boss_sq < 70.0 * 70.0);

            if is_parried {
                proj.lifetime = 0.0;
                let spark_color = if has_infernal_liturgy {
                    ORANGE
                } else if has_divine_shield || has_holy_hammer || has_sacred_scriptures {
                    GOLD
                } else if has_aegis_orbiters || has_invincible_bastion {
                    Color::new(0.3, 0.9, 1.0, 1.0)
                } else {
                    SKYBLUE
                };
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
                    boss_hits.push((actual_dmg, packet.damage_type));

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

    for id in projectile_obelisk_contributions { world.record_structure_contribution(id); }

    // Применяем всплывающий урон по структурам
    for (pos, txt) in structure_proj_hits {
        world.add_floating_text(pos, txt, GOLD, 1.1);
    }
    for (pos, txt) in crit_texts {
        world.add_floating_text(pos, txt, GOLD, 1.25);
    }

    // Применяем собранные эффекты по боссу и частицы
    for (actual_dmg, dmg_type) in boss_hits {
        apply_damage_to_boss(world, actual_dmg, dmg_type, DamageSourceKind::Ranged);
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

    // Обработка цепочек молний от пассивок (TeslaCoil, StaticCore, UkuleleResonance) и оружия (ChainLightning, StormGodWrath)
    if !chain_lightning_events.is_empty() {
        world.push_audio(crate::audio::AudioEvent::LightningStrike);
    }
    for (start_pos, target_id, max_jumps, jump_rad, chain_dmg, chain_type) in chain_lightning_events {
        let mut hit_ids = std::collections::HashSet::new();
        hit_ids.insert(target_id);
        let mut curr_pos = start_pos;

        for _ in 0..max_jumps {
            world.spatial_grid.query_nearby_into(curr_pos, jump_rad, &mut nearby_scratch);
            let mut best_target = None;
            let mut min_d_sq = jump_rad * jump_rad;

            for &idx in &nearby_scratch {
                if let Some(other) = world.mobs.get(idx) {
                    if other.health.is_alive() && !hit_ids.contains(&other.id) {
                        let d_sq = other.pos.distance_squared(curr_pos);
                        if d_sq < min_d_sq {
                            min_d_sq = d_sq;
                            best_target = Some((other.id, other.pos, idx));
                        }
                    }
                }
            }

            if let Some((t_id, t_pos, idx)) = best_target {
                hit_ids.insert(t_id);
                world.lightning_arcs.push((curr_pos, t_pos, 0.22));
                if let Some(target_mob) = world.mobs.get_mut(idx) {
                    let packet = DamagePacket {
                        source_id: Some(0),
                        target_id: t_id,
                        amount: chain_dmg,
                        damage_type: chain_type,
                        armor_penetration: 0.8,
                    };
                    let dmg = target_mob.resistances.calculate_damage(&packet, target_mob.is_undead);
                    target_mob.health.take_damage(dmg);
                    if !target_mob.knockback_immune {
                        let push_dir = (t_pos - curr_pos).normalize_or_zero();
                        target_mob.knockback_vel += push_dir * 90.0;
                    }
                }
                curr_pos = t_pos;
            } else {
                break;
            }
        }
    }

    // Обработка взрывов Осадной Мортиры (SiegeMortar)
    if !mortar_explosions.is_empty() {
        world.push_audio(crate::audio::AudioEvent::FireBlast);
    }
    for (exp_pos, exp_rad, exp_dmg) in mortar_explosions {
        world.spawn_particles(exp_pos, 16, ORANGE, exp_rad);
        world.spatial_grid.query_nearby_into(exp_pos, exp_rad, &mut nearby_scratch);
        for &idx in &nearby_scratch {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.health.is_alive() && mob.pos.distance_squared(exp_pos) < exp_rad * exp_rad {
                    let packet = DamagePacket {
                        source_id: Some(0),
                        target_id: mob.id,
                        amount: exp_dmg,
                        damage_type: DamageType::Fire,
                        armor_penetration: 0.4,
                    };
                    let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                    mob.health.take_damage(dmg);
                }
            }
        }
    }

    // Удаляем израсходованные снаряды
    world.projectiles.retain(|p| p.lifetime > 0.0 && p.pierce > 0);

    // 4. Смерть мобов, взрыв трупов (Hellfire) и дроп кристаллов душ
    let mut corpse_explosions = Vec::new();
    let mut dropped_gems = Vec::new();
    let mut spawned_phantoms = Vec::new();

    world.mobs.retain(|mob| {
        if !mob.health.is_alive() {
            // Пассивка "Жатва Душ": шанс призвать мстительных призраков
            if has_soul_harvester && world.boss_inventory.roll_luck(0.20) {
                spawned_phantoms.push(mob.pos);
            }

            // Эволюция "Адское Пекло": Взрыв трупов горящих врагов!
            if has_hellfire && mob.status.burn_timer > 0.0 {
                corpse_explosions.push((mob.pos, 55.0, 75.0));
            }

            // Шанс дропа особого артефакта "Магнит" (1.5%, а с элиты 8%)
            let magnet_chance = if mob.is_elite { 0.08 } else { 0.015 };
            let is_magnet = rand::gen_range(0.0, 1.0) < magnet_chance;

            let mut xp_value = match mob.tier {
                1 => 3.6,
                2 => 9.0,
                3 => 20.0,
                4 => 58.0,
                5 => 72.0,
                6 => 108.0,
                7 => 190.0,
                _ => 540.0,
            };

            if mob.is_elite {
                xp_value *= 2.5;
            }

            xp_value *= curse_xp_mult;

            if has_soul_feast {
                world.boss_health.heal(3.5);
                xp_value *= 1.25;
            }

            let martyr_lvl = world.boss_inventory.get_passive_level(PassiveType::MartyrVanguard);
            if martyr_lvl > 0 && world.boss_health.is_alive() {
                world.boss_health.heal(1.5 * martyr_lvl as f32);
            }
            for coop in world.coop_bosses.iter_mut() {
                let coop_martyr = coop.boss_inventory.get_passive_level(PassiveType::MartyrVanguard);
                if coop_martyr > 0 && coop.boss_health.is_alive() {
                    coop.boss_health.heal(1.5 * coop_martyr as f32);
                }
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

    // Призыв призраков от пассивки "Жатва Душ" (SoulHarvester)
    for p_pos in spawned_phantoms {
        let angle = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
        let dir = Vec2::new(angle.cos(), angle.sin());
        let pid = world.alloc_id();
        world.projectiles.push(crate::ecs::world::ProjectileEntity {
            id: pid,
            pos: p_pos,
            vel: dir * 320.0,
            damage: 42.0 * dmg_mult,
            damage_type: DamageType::True,
            armor_penetration: 0.6,
            lifetime: 2.8,
            radius: 14.0,
            pierce: 3,
            faction: Faction::Boss,
            source_weapon: Some(WeaponType::SoulPhantoms),
            homing_strength: 7.5,
        });
        world.spawn_particles(p_pos, 10, MAGENTA, 40.0);
    }

    // 4.1 Разрушение структур (Алтари Крови, Стены-Баррикады, Обелиски)
    let obelisk_reset_hp = crate::content::structures::OBELISK_BASE_HP_PER_BOSS * world.living_boss_count().max(1) as f32;
    let mut destroyed_events = Vec::new();
    world.structures.retain_mut(|st| {
        if !st.health.is_alive() {
            if st.structure_type == crate::content::structures::STRUCTURE_OBELISK {
                if st.is_active {
                    // Босс успел уничтожить Разлом до истечения времени зарядки (3 минуты)!
                    st.is_active = false;
                    st.health.max = obelisk_reset_hp;
                    st.health.current = st.health.max;
                    st.state_timer = 0.0;
                    destroyed_events.push((st.id, st.pos, crate::content::structures::STRUCTURE_OBELISK));
                }
                // Обелиск не удаляется из мира, остаётся в дремлющем виде
                true
            } else {
                destroyed_events.push((st.id, st.pos, st.structure_type));
                false
            }
        } else {
            true
        }
    });

    for (st_id, st_pos, st_type) in destroyed_events {
        match st_type {
            altar_type if crate::content::structures::is_altar_structure_type(altar_type) => {
                let altar_tier = crate::content::structures::altar_tier_from_structure_type(altar_type).unwrap_or(1);
                world.boss_altars_destroyed_total += 1;
                world.screen_shake = 14.0;
                world.spawn_particles(st_pos, 50, PURPLE, 190.0);
                world.spawn_particles(st_pos, 35, RED, 150.0);
                world.spawn_particles(st_pos, 25, GOLD, 130.0);
                world.add_floating_text(st_pos, format!("АЛТАРЬ ТИРА {} УНИЧТОЖЕН!", altar_tier), GOLD, 1.6);

                let total_altar_xp = 150.0 + altar_tier.saturating_sub(1) as f32 * 40.0;
                world.structure_contributors.remove(&st_id);
                world.grant_shared_boss_xp(total_altar_xp);
                world.add_floating_text(
                    st_pos,
                    format!("АЛТАРЬ ТИРА {} УНИЧТОЖЕН! +{:.0} XP ВСЕМ!", altar_tier, total_altar_xp),
                    GOLD,
                    1.8,
                );

                // Дропаем утилитный магнит без дублирования опыта
                dropped_gems.push(SoulGemEntity {
                    id: st_id,
                    pos: st_pos,
                    xp_value: 0.0,
                    is_magnet: true,
                });
            }
            crate::content::structures::STRUCTURE_WALL => {
                world.screen_shake = 6.0;
                world.spawn_particles(st_pos, 16, DARKGRAY, 90.0);
                world.spawn_particles(st_pos, 8, ORANGE, 70.0);
                world.add_floating_text(st_pos, "БАРЬЕР СЛОМАН!".to_string(), LIGHTGRAY, 1.1);
                dropped_gems.push(SoulGemEntity {
                    id: st_id,
                    pos: st_pos,
                    xp_value: 15.0,
                    is_magnet: false,
                });
            }
            crate::content::structures::STRUCTURE_OBELISK => {
                world.boss_obelisks_deactivated_total += 1;

                world.structure_contributors.remove(&st_id);
                world.obelisk_contributors.remove(&st_id);

                let all_boss_ids: Vec<u8> = std::iter::once(world.active_boss_id)
                    .chain(world.coop_bosses.iter().map(|p| p.id))
                    .collect();

                for boss_id in all_boss_ids {
                    if world.active_boss_id == boss_id {
                        if world.boss_health.is_alive() {
                            let heal_bonus = world.boss_health.max * 0.15;
                            world.boss_health.heal(heal_bonus);
                        }
                        if world.boss_human_controlled {
                            world.boss_pending_relic_choices += 1;
                            world.add_floating_text(
                                world.boss_pos,
                                "⚡ РАЗЛОМ ЗАКРЫТ! ВЫБЕРИТЕ РЕЛИКВИЮ!".to_string(),
                                GOLD,
                                3.0,
                            );
                        } else {
                            let granted_relic = grant_boss_relic_reward(world, boss_id);
                            let relic_name = granted_relic
                                .map(|r| crate::content::passives::get_passive_def(r).name)
                                .unwrap_or("Усиление Характеристик");
                            world.add_floating_text(
                                world.boss_pos,
                                format!("⚡ ДАР РАЗЛОМА: [{}]!", relic_name),
                                GOLD,
                                3.0,
                            );
                        }
                    } else {
                        let (is_alive, is_human, player_pos) = if let Some(player) = world.coop_bosses.iter().find(|p| p.id == boss_id) {
                            (player.boss_health.is_alive(), player.human_controlled, player.boss_pos)
                        } else {
                            continue;
                        };
                        if is_alive {
                            if let Some(player) = world.coop_bosses.iter_mut().find(|p| p.id == boss_id) {
                                let heal_bonus = player.boss_health.max * 0.15;
                                player.boss_health.heal(heal_bonus);
                            }
                        }
                        if is_human {
                            if let Some(player) = world.coop_bosses.iter_mut().find(|p| p.id == boss_id) {
                                player.pending_relic_choices += 1;
                            }
                            world.add_floating_text(
                                player_pos,
                                "⚡ РАЗЛОМ ЗАКРЫТ! ВЫБЕРИТЕ РЕЛИКВИЮ!".to_string(),
                                GOLD,
                                3.0,
                            );
                        } else {
                            let granted_relic = grant_boss_relic_reward(world, boss_id);
                            let relic_name = granted_relic
                                .map(|r| crate::content::passives::get_passive_def(r).name)
                                .unwrap_or("Усиление Характеристик");
                            world.add_floating_text(
                                player_pos,
                                format!("⚡ ДАР РАЗЛОМА: [{}]!", relic_name),
                                GOLD,
                                3.0,
                            );
                        }
                    }
                }
                world.screen_shake = 25.0;
                world.spawn_particles(st_pos, 80, GOLD, 260.0);
                world.spawn_particles(st_pos, 60, PURPLE, 220.0);
                world.spawn_particles(st_pos, 40, SKYBLUE, 180.0);
                world.add_floating_text(
                    st_pos,
                    "ИНФЕРНАЛЬНЫЙ РАЗЛОМ ЗАКРЫТ! ВЫБЕРИТЕ РЕЛИКВИЮ!".to_string(),
                    GOLD,
                    3.2,
                );
                // Дропаем утилитный магнит без повторного начисления XP (xp_value: 0.0)
                dropped_gems.push(SoulGemEntity {
                    id: st_id,
                    pos: st_pos,
                    xp_value: 0.0,
                    is_magnet: true,
                });
            }
            _ => {}
        }
    }

    // Обработка взрывов трупов от Hellfire
    for (pos, dmg, radius) in corpse_explosions {
        world.spawn_particles(pos, 18, ORANGE, 140.0);
        world.spatial_grid.query_nearby_into(pos, radius, &mut nearby_scratch);
        for &idx in &nearby_scratch {
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
        world.grant_shared_boss_xp(collected_xp);
    }
    crate::ecs::systems::coop::apply_pending_level_ups(world);

    // 6. Проверка Эгиды Бессмертия (ImmortalAegis Revive):
    if !world.boss_health.is_alive() && has_immortal_aegis && !world.boss_inventory.has_used_aegis_revive {
        world.boss_inventory.has_used_aegis_revive = true;
        world.boss_health.current = world.boss_health.max;
        world.boss_invulnerable_timer = 3.0; // 3 сек абсолютной неуязвимости
        world.add_floating_text(
            boss_pos,
            "ВОСКРЕШЕНИЕ: ЭГИДА БЕССМЕРТИЯ!".to_string(),
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
    // 1. O(N) слияние близких кристаллов через хеш-таблицу ячеек 70x70 px.
    // Опыт не теряется: XP поглощённого кристалла переходит представителю ячейки.
    let cell = merge_dist.max(1.0);
    let mut cell_representative: std::collections::HashMap<(usize, usize), usize> = std::collections::HashMap::with_capacity(len);

    for i in 0..len {
        if gems[i].is_magnet || gems[i].xp_value <= 0.0 {
            continue;
        }
        if gems[i].pos.distance_squared(boss_pos) < boss_dist_min_sq {
            continue;
        }

        let cx = (gems[i].pos.x.max(0.0) / cell).floor() as usize;
        let cy = (gems[i].pos.y.max(0.0) / cell).floor() as usize;

        if let Some(&rep) = cell_representative.get(&(cx, cy)) {
            gems[rep].xp_value += gems[i].xp_value;
            gems[i].xp_value = 0.0;
        } else {
            cell_representative.insert((cx, cy), i);
        }
    }

    gems.retain(|g| g.is_magnet || g.xp_value > 0.0);

    // 2. Если кристаллов всё ещё больше целевого капа, укрупняем сетку до 140x140 px
    // Это сохраняет 100% пространственную локальность (сливаются ТОЛЬКО соседние кристаллы, без телепортации!)
    if gems.len() > target_max {
        let cell_large = 140.0f32;
        let len_l = gems.len();
        let mut large_rep: std::collections::HashMap<(usize, usize), usize> = std::collections::HashMap::with_capacity(len_l);

        for i in 0..len_l {
            if gems[i].is_magnet || gems[i].xp_value <= 0.0 {
                continue;
            }
            if gems[i].pos.distance_squared(boss_pos) < boss_dist_min_sq {
                continue;
            }
            let cx = (gems[i].pos.x.max(0.0) / cell_large).floor() as usize;
            let cy = (gems[i].pos.y.max(0.0) / cell_large).floor() as usize;

            if let Some(&rep) = large_rep.get(&(cx, cy)) {
                gems[rep].xp_value += gems[i].xp_value;
                gems[i].xp_value = 0.0;
            } else {
                large_rep.insert((cx, cy), i);
            }
        }
        gems.retain(|g| g.is_magnet || g.xp_value > 0.0);
    }
}

fn grant_boss_relic_reward(
    world: &mut World,
    player_id: u8,
) -> Option<PassiveType> {
    if world.active_boss_id == player_id {
        grant_boss_relic_internal(world)
    } else if let Some(idx) = world.coop_bosses.iter().position(|p| p.id == player_id) {
        if world.swap_coop_boss_context(idx) {
            let res = grant_boss_relic_internal(world);
            world.swap_coop_boss_context(idx);
            res
        } else {
            None
        }
    } else {
        None
    }
}

fn grant_boss_relic_internal(world: &mut World) -> Option<PassiveType> {
    let options = crate::ecs::systems::boss_ai::generate_relic_choices(&world.boss_inventory);
    if let Some(choice) = options.first().cloned() {
        crate::ecs::systems::boss_ai::apply_perk_choice(world, choice.clone());
        match choice {
            crate::ecs::components::inventory::PerkChoice::UpgradePassive(k) => Some(k),
            _ => None,
        }
    } else {
        // Все слоты заполнены и прокачаны до 5-го уровня: альтернативное усиление характеристик
        world.boss_health.max += 250.0;
        world.boss_health.heal(250.0);
        world.boss_resistances.flat_armor += 3.0;
        None
    }
}
