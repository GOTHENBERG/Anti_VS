use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::components::combat::{DamagePacket, DamageTarget, DamageType, Faction, Health, Resistances};
use crate::ecs::components::inventory::{BossInventory, DamageSourceKind, PassiveType, WeaponType};
use crate::ecs::world::{BossPlayerState, MobEntity, SoulGemEntity, World};

#[derive(Clone, Copy)]
enum BossSlot {
    Primary,
    Coop(usize),
}

struct BossCombatProfile {
    slot: BossSlot,
    boss_id: u8,
    pos: Vec2,
    is_invulnerable: bool,
    has_blade_squall: bool,
    has_holy_hammer: bool,
    has_light_bastion: bool,
    has_nether_blaze: bool,
    has_aegis_orbiters: bool,
    has_invincible_bastion: bool,
    has_sacred_scriptures: bool,
    has_infernal_liturgy: bool,
    has_occult_prism: bool,
    has_pyromaniac: bool,
    has_soul_feast: bool,
    has_static_core: bool,
    has_ukulele: bool,
    has_calamity_mortar: bool,
    has_soul_harvester: bool,
    tesla_chance: f32,
    crit_chance: f32,
    dmg_mult: f32,
    curse_xp_mult: f32,
}

impl BossCombatProfile {
    fn new(
        slot: BossSlot,
        boss_id: u8,
        pos: Vec2,
        invuln_timer: f32,
        inv: &BossInventory,
    ) -> Self {
        Self {
            slot,
            boss_id,
            pos,
            is_invulnerable: invuln_timer > 0.0,
            has_blade_squall: inv.has_weapon(WeaponType::BladeSquall),
            has_holy_hammer: inv.has_weapon(WeaponType::HolyHammer),
            has_light_bastion: inv.has_weapon(WeaponType::LightBastion),
            has_nether_blaze: inv.has_weapon(WeaponType::NetherBlaze),
            has_aegis_orbiters: inv.has_weapon(WeaponType::AegisOrbiters),
            has_invincible_bastion: inv.has_weapon(WeaponType::InvincibleBastion),
            has_sacred_scriptures: inv.has_weapon(WeaponType::SacredScriptures),
            has_infernal_liturgy: inv.has_weapon(WeaponType::InfernalLiturgy),
            has_occult_prism: inv.has_passive(PassiveType::OccultPrism),
            has_pyromaniac: inv.has_passive(PassiveType::Pyromaniac),
            has_soul_feast: inv.has_weapon(WeaponType::SoulFeast),
            has_static_core: inv.has_passive(PassiveType::StaticCore),
            has_ukulele: inv.has_passive(PassiveType::UkuleleResonance),
            has_calamity_mortar: inv.has_passive(PassiveType::CalamityMortar),
            has_soul_harvester: inv.has_passive(PassiveType::SoulHarvester),
            tesla_chance: inv.get_tesla_chain_chance(),
            crit_chance: inv.get_crit_chance(),
            dmg_mult: inv.get_damage_multiplier(),
            curse_xp_mult: inv.get_curse_stone_xp_multiplier(),
        }
    }

    fn parry_spark_color(&self, dist_sq: f32) -> Option<Color> {
        let is_parried = (self.has_blade_squall && dist_sq < 85.0 * 85.0)
            || (self.has_holy_hammer && dist_sq < 88.0 * 88.0)
            || (self.has_light_bastion && dist_sq < 98.0 * 98.0)
            || (self.has_aegis_orbiters && dist_sq < 106.0 * 106.0)
            || (self.has_invincible_bastion && dist_sq < 125.0 * 125.0)
            || (self.has_sacred_scriptures && dist_sq < 92.0 * 92.0)
            || (self.has_infernal_liturgy && dist_sq < 115.0 * 115.0)
            || (self.is_invulnerable && dist_sq < 70.0 * 70.0);
        if !is_parried {
            return None;
        }
        let color = if self.has_infernal_liturgy {
            ORANGE
        } else if self.has_light_bastion || self.has_holy_hammer || self.has_sacred_scriptures {
            GOLD
        } else if self.has_aegis_orbiters || self.has_invincible_bastion {
            Color::new(0.3, 0.9, 1.0, 1.0)
        } else {
            SKYBLUE
        };
        Some(color)
    }

    fn orbital_mob_auras(&self) -> [(bool, f32, f32, DamageType, f32, f32, bool); 7] {
        [
            (self.has_blade_squall, 85.0, 65.0, DamageType::Physical, 0.35, 180.0, false),
            (self.has_holy_hammer, 88.0, 60.0, DamageType::Holy, 0.25, 0.0, false),
            (self.has_light_bastion, 98.0, 80.0, DamageType::Holy, 0.20, 0.0, true),
            (self.has_aegis_orbiters, 106.0, 70.0, DamageType::Physical, 0.35, 0.0, false),
            (self.has_invincible_bastion, 125.0, 115.0, DamageType::Holy, 0.50, 0.0, false),
            (self.has_sacred_scriptures, 92.0, 55.0, DamageType::Holy, 0.30, 0.0, false),
            (self.has_infernal_liturgy, 115.0, 110.0, DamageType::Fire, 0.50, 0.0, false),
        ]
    }

    fn orbital_structure_auras(&self) -> [(bool, f32, f32, DamageType, f32, bool); 8] {
        [
            (self.has_blade_squall, 85.0, 75.0, DamageType::Physical, 0.35, false),
            (self.has_holy_hammer, 88.0, 70.0, DamageType::Holy, 0.25, false),
            (self.has_light_bastion, 98.0, 90.0, DamageType::Holy, 0.20, true),
            (self.has_nether_blaze, 95.0, 85.0, DamageType::Fire, 0.25, false),
            (self.has_aegis_orbiters, 106.0, 80.0, DamageType::Physical, 0.35, false),
            (self.has_invincible_bastion, 125.0, 110.0, DamageType::Holy, 0.50, false),
            (self.has_sacred_scriptures, 92.0, 65.0, DamageType::Holy, 0.30, false),
            (self.has_infernal_liturgy, 115.0, 120.0, DamageType::Fire, 0.50, false),
        ]
    }
}

fn collect_live_boss_profiles(world: &World) -> Vec<BossCombatProfile> {
    let mut profiles = Vec::with_capacity(1 + world.coop_bosses.len());
    if world.boss_health.is_alive() {
        profiles.push(BossCombatProfile::new(
            BossSlot::Primary,
            world.active_boss_id,
            world.boss_pos,
            world.boss_invulnerable_timer,
            &world.boss_inventory,
        ));
    }
    for (idx, player) in world.coop_bosses.iter().enumerate() {
        if player.boss_health.is_alive() {
            profiles.push(BossCombatProfile::new(
                BossSlot::Coop(idx),
                player.id,
                player.boss_pos,
                player.boss_invulnerable_timer,
                &player.boss_inventory,
            ));
        }
    }
    profiles
}

fn boss_slot_by_id_parts(
    active_boss_id: u8,
    coop_bosses: &[BossPlayerState],
    boss_id: u8,
) -> Option<BossSlot> {
    if active_boss_id == boss_id {
        Some(BossSlot::Primary)
    } else {
        coop_bosses
            .iter()
            .position(|p| p.id == boss_id)
            .map(BossSlot::Coop)
    }
}

fn roll_slot_luck(
    slot: Option<BossSlot>,
    chance: f32,
    primary_inv: &mut BossInventory,
    coop_bosses: &mut [BossPlayerState],
) -> bool {
    if chance <= 0.0 {
        return false;
    }
    match slot {
        Some(BossSlot::Primary) => primary_inv.roll_luck(chance),
        Some(BossSlot::Coop(idx)) => coop_bosses
            .get_mut(idx)
            .is_some_and(|p| p.boss_inventory.roll_luck(chance)),
        None => false,
    }
}

fn heal_boss_slot_parts(
    slot: Option<BossSlot>,
    amount: f32,
    primary_hp: &mut Health,
    coop_bosses: &mut [BossPlayerState],
) {
    if amount <= 0.0 {
        return;
    }
    match slot {
        Some(BossSlot::Primary) => {
            if primary_hp.is_alive() {
                primary_hp.heal(amount);
            }
        }
        Some(BossSlot::Coop(idx)) => {
            if let Some(p) = coop_bosses.get_mut(idx) {
                if p.boss_health.is_alive() {
                    p.boss_health.heal(amount);
                }
            }
        }
        None => {}
    }
}

fn boss_slot_resistances(world: &World, slot: BossSlot) -> &Resistances {
    match slot {
        BossSlot::Primary => &world.boss_resistances,
        BossSlot::Coop(idx) => &world.coop_bosses[idx].boss_resistances,
    }
}

fn boss_slot_is_invulnerable(world: &World, slot: BossSlot) -> bool {
    match slot {
        BossSlot::Primary => !world.boss_health.is_alive() || world.boss_invulnerable_timer > 0.0,
        BossSlot::Coop(idx) => world
            .coop_bosses
            .get(idx)
            .is_none_or(|p| !p.boss_health.is_alive() || p.boss_invulnerable_timer > 0.0),
    }
}

fn apply_pestilence_poison_to_slot(world: &mut World, slot: BossSlot, level: u32, chance_per_lvl: f32, max_chance: f32) {
    if level == 0 {
        return;
    }
    let poison_chance = ((level as f32) * chance_per_lvl).min(max_chance);
    if rand::gen_range(0.0, 1.0) >= poison_chance {
        return;
    }
    let dps = 6.0 + (level as f32) * 2.2;
    match slot {
        BossSlot::Primary => {
            world.boss_poison_timer = 2.5;
            world.boss_poison_dps = world.boss_poison_dps.max(dps);
        }
        BossSlot::Coop(idx) => {
            if let Some(player) = world.coop_bosses.get_mut(idx) {
                player.boss_poison_timer = 2.5;
                player.boss_poison_dps = player.boss_poison_dps.max(dps);
            }
        }
    }
}

#[inline(always)]
fn apply_radial_hit_to_mob(
    mob: &mut MobEntity,
    center: Vec2,
    radius_sq: f32,
    damage: f32,
    knockback_impulse: f32,
    owner_boss_id: Option<u8>,
) {
    let diff = mob.pos - center;
    let dist_sq = diff.length_squared();
    if dist_sq < radius_sq {
        mob.health.take_damage(damage);
        if owner_boss_id.is_some() && damage > 0.0 {
            mob.last_hit_boss_id = owner_boss_id;
        }
        if knockback_impulse > 0.0 && dist_sq > 1.0 && !mob.knockback_immune {
            mob.vel += (diff / dist_sq.sqrt()) * knockback_impulse;
        }
    }
}

/// Наносит радиальный урон (и опциональный импульс отбрасывания) через SpatialGrid без перебора 20к мобов.
fn apply_radial_mob_burst(
    world: &mut World,
    center: Vec2,
    radius: f32,
    damage: f32,
    knockback_impulse: f32,
    owner_boss_id: Option<u8>,
) {
    let radius_sq = radius * radius;
    if !world.spatial_grid.is_empty() && world.spatial_grid.len() == world.mobs.len() {
        let pad = Vec2::splat(radius);
        let spatial_grid = &world.spatial_grid;
        let mobs = &mut world.mobs;
        spatial_grid.query_aabb_unique(center - pad, center + pad, |mob_idx| {
            if let Some(mob) = mobs.get_mut(mob_idx) {
                apply_radial_hit_to_mob(mob, center, radius_sq, damage, knockback_impulse, owner_boss_id);
            }
        });
    } else {
        for mob in &mut world.mobs {
            apply_radial_hit_to_mob(mob, center, radius_sq, damage, knockback_impulse, owner_boss_id);
        }
    }
}

fn apply_damage_to_boss_slot(
    world: &mut World,
    slot: BossSlot,
    mut amount: f32,
    damage_type: DamageType,
    source_kind: DamageSourceKind,
) {
    if amount <= 0.0 {
        return;
    }

    let (boss_id, boss_pos, has_shroud, barrier_triggered) = match slot {
        BossSlot::Primary => {
            let shroud = world.boss_inventory.has_weapon(WeaponType::BloodEmperorShroud);
            let triggered = if world.boss_inventory.astral_cloak_charges > 0 {
                world.boss_inventory.astral_cloak_charges -= 1;
                true
            } else {
                false
            };
            (world.active_boss_id, world.boss_pos, shroud, triggered)
        }
        BossSlot::Coop(idx) => {
            let Some(player) = world.coop_bosses.get_mut(idx) else {
                return;
            };
            let shroud = player.boss_inventory.has_weapon(WeaponType::BloodEmperorShroud);
            let triggered = if player.boss_inventory.astral_cloak_charges > 0 {
                player.boss_inventory.astral_cloak_charges -= 1;
                true
            } else {
                false
            };
            (player.id, player.boss_pos, shroud, triggered)
        }
    };

    if barrier_triggered {
        world.add_floating_text(boss_pos, "БАРЬЕР".to_string(), SKYBLUE, 1.0);
        if has_shroud {
            world.spawn_particles(boss_pos, 35, RED, 220.0);
            apply_radial_mob_burst(world, boss_pos, 220.0, 200.0, 0.0, Some(boss_id));
        }
        return;
    }

    if has_shroud {
        amount = amount.min(12.0);
        world.spawn_particles(boss_pos, 15, MAROON, 150.0);
        apply_radial_mob_burst(world, boss_pos, 150.0, 75.0, 0.0, Some(boss_id));
    }

    match slot {
        BossSlot::Primary => {
            world.boss_health.take_damage(amount);
            world.boss_inventory.record_damage_taken(amount, damage_type, source_kind);
            world.total_damage_dealt_to_boss += amount;
            world.boss_damage_taken_timer = 3.0;
            world.boss_still_time = 0.0;
        }
        BossSlot::Coop(idx) => {
            if let Some(player) = world.coop_bosses.get_mut(idx) {
                player.boss_health.take_damage(amount);
                player.boss_inventory.record_damage_taken(amount, damage_type, source_kind);
                world.total_damage_dealt_to_boss += amount;
                player.boss_damage_taken_timer = 3.0;
                player.boss_still_time = 0.0;
            }
        }
    }
}

pub fn update_combat(world: &mut World, dt: f32) {
    if world.spatial_grid.len() != world.mobs.len() {
        world.rebuild_spatial_grid();
    }

    let profiles = collect_live_boss_profiles(world);
    let boss_radius = 24.0;
    let is_reward_structure = |st_type: u8| {
        crate::content::structures::is_altar_structure_type(st_type)
            || st_type == crate::content::structures::STRUCTURE_OBELISK
    };
    let pestilence_lvl = world.directors.first().map_or(0, |d| d.grade_pestilence);

    let mut kamikaze_explosions = Vec::new();
    let mut boss_contact_hits: Vec<(BossSlot, f32, DamageType)> = Vec::new();
    let mut contact_poison_slots: Vec<BossSlot> = Vec::new();
    let mut nearby_scratch: Vec<usize> = Vec::with_capacity(128);

    // 1. Коллизии мобов с Боссами, орбиталями и контактный урон
    let mut contact_kamikaze_blast = false;
    for profile in &profiles {
        let auras = profile.orbital_mob_auras();
        world.spatial_grid.query_nearby_into(profile.pos, 120.0, &mut nearby_scratch);
        for &idx in &nearby_scratch {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if !mob.health.is_alive() {
                    continue;
                }
                let dist_sq = mob.pos.distance_squared(profile.pos);
                let collision_dist = mob.radius + boss_radius;

                if dist_sq < collision_dist * collision_dist {
                    if mob.tier == 5 {
                        kamikaze_explosions.push((mob.pos, 220.0, 95.0));
                        mob.health.take_damage(9999.0);
                        contact_kamikaze_blast = true;
                    } else if !profile.is_invulnerable {
                        let packet = DamagePacket::boss(
                            Some(mob.id),
                            mob.damage * dt,
                            DamageType::Physical,
                            0.0,
                        );
                        let res = match profile.slot {
                            BossSlot::Primary => &world.boss_resistances,
                            BossSlot::Coop(c_idx) => &world.coop_bosses[c_idx].boss_resistances,
                        };
                        let actual_dmg = res.calculate_damage(&packet, false);
                        boss_contact_hits.push((profile.slot, actual_dmg, packet.damage_type));
                        if pestilence_lvl > 0 {
                            contact_poison_slots.push(profile.slot);
                        }
                        if mob.tier == 1 {
                            match profile.slot {
                                BossSlot::Primary => world.boss_vel *= 0.98,
                                BossSlot::Coop(c_idx) => {
                                    if let Some(p) = world.coop_bosses.get_mut(c_idx) {
                                        p.boss_vel *= 0.98;
                                    }
                                }
                            }
                        }
                    }
                }

                for (enabled, rad, dps, dmg_type, pen, kb, lifesteal) in auras {
                    if !enabled || dist_sq >= rad * rad {
                        continue;
                    }
                    let packet = DamagePacket::mob(
                        Some(0),
                        idx,
                        mob.id,
                        dps * dt * profile.dmg_mult,
                        dmg_type,
                        pen,
                    )
                    .with_owner(Some(profile.boss_id));
                    let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                    mob.health.take_damage(dmg);
                    if dmg > 0.0 {
                        mob.last_hit_boss_id = Some(profile.boss_id);
                    }
                    if kb > 0.0 && !mob.knockback_immune {
                        let push_dir = (mob.pos - profile.pos).normalize_or_zero();
                        mob.knockback_vel += push_dir * kb;
                    }
                    if lifesteal {
                        heal_boss_slot_parts(
                            Some(profile.slot),
                            dmg * 0.05,
                            &mut world.boss_health,
                            &mut world.coop_bosses,
                        );
                    }
                }
            }
        }
    }

    for (slot, actual_dmg, dmg_type) in boss_contact_hits {
        apply_damage_to_boss_slot(world, slot, actual_dmg, dmg_type, DamageSourceKind::Melee);
    }
    for slot in contact_poison_slots {
        apply_pestilence_poison_to_slot(world, slot, pestilence_lvl, 0.05, 0.40);
    }

    if contact_kamikaze_blast {
        world.push_audio(crate::audio::AudioEvent::FireBlast);
    }

    // 1.3 Контактный урон аур по разрушаемым Алтарям Крови и Обелискам
    let mut structure_contribution_ids: Vec<(u8, usize)> = Vec::new();
    for profile in &profiles {
        let auras = profile.orbital_structure_auras();
        for st in &mut world.structures {
            if !crate::content::structures::boss_can_damage_structure(st) {
                continue;
            }
            let dist_sq = st.pos.distance_squared(profile.pos);
            for (enabled, rad, dps, dmg_type, pen, lifesteal) in auras {
                let threshold = rad + st.radius;
                if !enabled || dist_sq >= threshold * threshold {
                    continue;
                }
                let packet = DamagePacket::structure(Some(0), st.id, dps * dt, dmg_type, pen)
                    .with_owner(Some(profile.boss_id));
                let dmg = st.resistances.calculate_damage(&packet, false);
                st.health.take_damage(dmg);
                if is_reward_structure(st.structure_type) && dmg > 0.0 {
                    structure_contribution_ids.push((profile.boss_id, st.id));
                }
                if lifesteal {
                    heal_boss_slot_parts(
                        Some(profile.slot),
                        dmg * 0.05,
                        &mut world.boss_health,
                        &mut world.coop_bosses,
                    );
                }
            }
        }
    }
    for (boss_id, st_id) in structure_contribution_ids.drain(..) {
        world.record_structure_contribution_for(boss_id, st_id);
    }

    // Обработка детонаций Камикадзе
    for (exp_pos, exp_damage, exp_radius) in kamikaze_explosions {
        world.screen_shake = 12.0;
        world.spawn_particles(exp_pos, 45, RED, 220.0);
        world.spawn_particles(exp_pos, 25, ORANGE, 180.0);

        let exp_rad_sq = exp_radius * exp_radius;
        for profile in &profiles {
            if !profile.is_invulnerable && exp_pos.distance_squared(profile.pos) < exp_rad_sq {
                let packet = DamagePacket::boss(None, exp_damage, DamageType::Fire, 0.25);
                let actual_dmg =
                    boss_slot_resistances(world, profile.slot).calculate_damage(&packet, false);
                apply_damage_to_boss_slot(
                    world,
                    profile.slot,
                    actual_dmg,
                    packet.damage_type,
                    DamageSourceKind::Hazard,
                );
                if !world.is_headless {
                    world.add_floating_text(profile.pos, format!("-{:.0}", actual_dmg), RED, 1.4);
                }
            }
        }

        // Взрыв также наносит урон окружающим мобам
        world.spatial_grid.query_nearby_into(exp_pos, exp_radius, &mut nearby_scratch);
        for &idx in &nearby_scratch {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(exp_pos) < exp_rad_sq {
                    mob.health.take_damage(exp_damage * 0.5);
                }
            }
        }
    }

    // 2. Обработка очереди урона (из skills, кислотных луж и т.д.)
    let mut queue = std::mem::take(&mut world.damage_queue);
    let mut structure_texts = Vec::new();
    let mut queued_obelisk_contributions: Vec<(u8, usize)> = Vec::new();

    for packet in queue.drain(..) {
        match packet.target {
            DamageTarget::Boss => {
                let slot = packet
                    .owner_boss_id
                    .and_then(|id| boss_slot_by_id_parts(world.active_boss_id, &world.coop_bosses, id))
                    .unwrap_or(BossSlot::Primary);
                if !boss_slot_is_invulnerable(world, slot) {
                    let actual_dmg =
                        boss_slot_resistances(world, slot).calculate_damage(&packet, false);
                    let source_kind = if packet.source_id.is_none() {
                        DamageSourceKind::Hazard
                    } else {
                        DamageSourceKind::Melee
                    };
                    apply_damage_to_boss_slot(world, slot, actual_dmg, packet.damage_type, source_kind);
                }
            }
            DamageTarget::Mob { index, id } => {
                let target_mob = if world.mobs.get(index).is_some_and(|m| m.id == id) {
                    world.mobs.get_mut(index)
                } else {
                    world.mobs.iter_mut().find(|m| m.id == id)
                };
                if let Some(mob) = target_mob {
                    let actual_dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                    mob.health.take_damage(actual_dmg);
                    if packet.owner_boss_id.is_some() && actual_dmg > 0.0 {
                        mob.last_hit_boss_id = packet.owner_boss_id;
                    }
                }
            }
            DamageTarget::Structure { id } => {
                if let Some(st) = world
                    .structures
                    .iter_mut()
                    .find(|s| s.id == id && crate::content::structures::boss_can_damage_structure(s))
                {
                    let actual_dmg = st.resistances.calculate_damage(&packet, false);
                    st.health.take_damage(actual_dmg);
                    if is_reward_structure(st.structure_type) && actual_dmg > 0.0 {
                        if let Some(owner_id) = packet.owner_boss_id {
                            queued_obelisk_contributions.push((owner_id, st.id));
                        }
                    }
                    if !world.is_headless {
                        structure_texts.push((st.pos, format!("-{:.0}", actual_dmg)));
                    }
                }
            }
        }
    }
    if world.damage_queue.is_empty() {
        world.damage_queue = queue;
    }
    for (owner_id, st_id) in queued_obelisk_contributions {
        world.record_structure_contribution_for(owner_id, st_id);
    }
    for (pos, txt) in structure_texts {
        world.add_floating_text(pos, txt, GOLD, 1.0);
    }

    // 3. Коллизии снарядов (swept-отрезки [prev_pos, pos] для защиты от туннелирования на 20 Гц)
    let mut boss_hits: Vec<(BossSlot, f32, DamageType, Vec2)> = Vec::new();
    let mut director_poison_slots: Vec<BossSlot> = Vec::new();
    let mut hit_particles = Vec::new();
    let mut crit_texts = Vec::new();
    let mut structure_proj_hits = Vec::new();
    let mut projectile_obelisk_contributions: Vec<(u8, usize)> = Vec::new();
    let mut chain_lightning_events: Vec<(Vec2, usize, usize, f32, f32, DamageType, Option<u8>)> = Vec::new();
    let mut mortar_explosions: Vec<(Vec2, f32, f32, Option<u8>)> = Vec::new();

    {
        let active_boss_id = world.active_boss_id;
        let is_headless = world.is_headless;
        let projectiles = &mut world.projectiles;
        let spatial_grid = &world.spatial_grid;
        let mobs = &mut world.mobs;
        let structures = &mut world.structures;
        let primary_inv = &mut world.boss_inventory;
        let primary_hp = &mut world.boss_health;
        let primary_res = &world.boss_resistances;
        let coop_bosses = &mut world.coop_bosses;

        for proj in projectiles.iter_mut() {
            if proj.lifetime <= 0.0 || proj.pierce == 0 {
                continue;
            }

            if proj.faction == Faction::Boss {
                let owner_id = proj.owner_boss_id;
                let owner_slot = owner_id
                    .and_then(|id| boss_slot_by_id_parts(active_boss_id, coop_bosses, id));
                let owner_profile = owner_id
                    .and_then(|id| profiles.iter().find(|p| p.boss_id == id));
                let crit_chance = owner_profile.map_or(0.0, |p| p.crit_chance);

                // Снаряды босса поражают мобов вдоль всего пройденного сегмента [prev_pos, pos]
                nearby_scratch.clear();
                spatial_grid.query_segment_thick(
                    proj.prev_pos,
                    proj.pos,
                    proj.radius + 30.0,
                    |idx| nearby_scratch.push(idx),
                );
                for &idx in &nearby_scratch {
                    if proj.pierce == 0 {
                        break;
                    }
                    if let Some(mob) = mobs.get_mut(idx) {
                        if !mob.health.is_alive() {
                            continue;
                        }
                        let (seg_dist, _) = crate::ecs::systems::coop::dist_to_segment(
                            mob.pos,
                            proj.prev_pos,
                            proj.pos,
                        );
                        if seg_dist < mob.radius + proj.radius {
                            let is_crit = crit_chance > 0.0
                                && roll_slot_luck(owner_slot, crit_chance, primary_inv, coop_bosses);
                            let base_damage = if is_crit { proj.damage * 2.0 } else { proj.damage };

                            let packet = DamagePacket::mob(
                                Some(0),
                                idx,
                                mob.id,
                                base_damage,
                                proj.damage_type,
                                proj.armor_penetration,
                            )
                            .with_owner(owner_id);
                            let actual_dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                            mob.health.take_damage(actual_dmg);
                            if owner_id.is_some() && actual_dmg > 0.0 {
                                mob.last_hit_boss_id = owner_id;
                            }
                            proj.pierce = proj.pierce.saturating_sub(1);
                            if proj.pierce > 0 {
                                proj.homing_strength = 0.0;
                            }

                            if is_crit && !is_headless {
                                hit_particles.push((mob.pos, GOLD));
                                crit_texts.push((mob.pos, format!("КРИТ {:.0}!", actual_dmg)));
                            }

                            if let Some(op) = owner_profile {
                                // Цепные молнии от пассивок владельца: Катушка Теслы и Статическое Ядро
                                if op.tesla_chance > 0.0
                                    && roll_slot_luck(owner_slot, op.tesla_chance, primary_inv, coop_bosses)
                                {
                                    let targets = if op.has_static_core { 6 } else { 3 };
                                    chain_lightning_events.push((
                                        mob.pos,
                                        mob.id,
                                        targets,
                                        230.0,
                                        38.0 * op.dmg_mult,
                                        DamageType::True,
                                        owner_id,
                                    ));
                                }

                                // Резонанс Укулеле владельца: 25% шанс электро-цепочки на 4 цели
                                if op.has_ukulele
                                    && roll_slot_luck(owner_slot, 0.25, primary_inv, coop_bosses)
                                {
                                    chain_lightning_events.push((
                                        mob.pos,
                                        mob.id,
                                        4,
                                        190.0,
                                        35.0 * op.dmg_mult,
                                        DamageType::True,
                                        owner_id,
                                    ));
                                }

                                // Мортира Бедствия владельца: взрыв 55px при прямом попадании (50% урона)
                                if op.has_calamity_mortar {
                                    mortar_explosions.push((mob.pos, 55.0, proj.damage * 0.5, owner_id));
                                }

                                // Вампиризм заклинаний от Оккультной Призмы (6%)
                                if op.has_occult_prism {
                                    heal_boss_slot_parts(owner_slot, actual_dmg * 0.06, primary_hp, coop_bosses);
                                }

                                // Вампиризм огня от Сердца Пироманта (8%)
                                if op.has_pyromaniac && proj.damage_type == DamageType::Fire {
                                    heal_boss_slot_parts(owner_slot, actual_dmg * 0.08, primary_hp, coop_bosses);
                                }
                            }

                            // Оружие: Цепная Молния (ChainLightning) — прыгает на 3 цели
                            if proj.source_weapon == Some(WeaponType::ChainLightning) {
                                chain_lightning_events.push((
                                    mob.pos,
                                    mob.id,
                                    3,
                                    200.0,
                                    actual_dmg * 0.75,
                                    DamageType::True,
                                    owner_id,
                                ));
                            }

                            // Оружие (Эволюция): Гнев Бури (StormGodWrath) — разряд на 6 целей
                            if proj.source_weapon == Some(WeaponType::StormGodWrath) {
                                chain_lightning_events.push((
                                    mob.pos,
                                    mob.id,
                                    6,
                                    260.0,
                                    actual_dmg * 1.0,
                                    DamageType::True,
                                    owner_id,
                                ));
                            }

                            // Вампиризм от Кровавой Жатвы (BloodHarvest): 20% владельцу
                            if proj.source_weapon == Some(WeaponType::BloodHarvest) {
                                heal_boss_slot_parts(owner_slot, actual_dmg * 0.20, primary_hp, coop_bosses);
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

                // Снаряды босса также атакуют и разрушают Алтари Крови и Обелиски
                for st in structures.iter_mut() {
                    if proj.pierce == 0 {
                        break;
                    }
                    if !st.health.is_alive() || !crate::content::structures::boss_can_damage_structure(st) {
                        continue;
                    }
                    let (seg_dist, _) = crate::ecs::systems::coop::dist_to_segment(
                        st.pos,
                        proj.prev_pos,
                        proj.pos,
                    );
                    if seg_dist < st.radius + proj.radius {
                        let packet = DamagePacket::structure(
                            Some(0),
                            st.id,
                            proj.damage,
                            proj.damage_type,
                            proj.armor_penetration,
                        )
                        .with_owner(owner_id);
                        let actual_dmg = st.resistances.calculate_damage(&packet, false);
                        st.health.take_damage(actual_dmg);
                        if is_reward_structure(st.structure_type) && actual_dmg > 0.0 {
                            if let Some(oid) = owner_id {
                                projectile_obelisk_contributions.push((oid, st.id));
                            }
                        }
                        proj.pierce = proj.pierce.saturating_sub(1);
                        if proj.pierce > 0 {
                            proj.homing_strength = 0.0;
                        }
                        if !is_headless {
                            hit_particles.push((st.pos, PURPLE));
                            structure_proj_hits.push((st.pos, format!("-{:.0}", actual_dmg)));
                        }
                    }
                }
            } else if proj.faction == Faction::Director {
                for profile in &profiles {
                    let (seg_dist, _) = crate::ecs::systems::coop::dist_to_segment(
                        profile.pos,
                        proj.prev_pos,
                        proj.pos,
                    );
                    let dist_to_boss_sq = seg_dist * seg_dist;

                    if let Some(spark_color) = profile.parry_spark_color(dist_to_boss_sq) {
                        proj.lifetime = 0.0;
                        proj.pierce = 0;
                        hit_particles.push((proj.pos, spark_color));
                        break;
                    }

                    if seg_dist < boss_radius + proj.radius {
                        proj.lifetime = 0.0;
                        proj.pierce = 0;
                        if !profile.is_invulnerable {
                            let packet = DamagePacket::boss(
                                Some(proj.id),
                                proj.damage,
                                proj.damage_type,
                                proj.armor_penetration,
                            );
                            let res = match profile.slot {
                                BossSlot::Primary => primary_res,
                                BossSlot::Coop(c_idx) => &coop_bosses[c_idx].boss_resistances,
                            };
                            let actual_dmg = res.calculate_damage(&packet, false);
                            boss_hits.push((profile.slot, actual_dmg, packet.damage_type, profile.pos));
                            if pestilence_lvl > 0 {
                                director_poison_slots.push(profile.slot);
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    for (owner_id, st_id) in projectile_obelisk_contributions {
        world.record_structure_contribution_for(owner_id, st_id);
    }

    // Применяем всплывающий урон по структурам и криты
    for (pos, txt) in structure_proj_hits {
        world.add_floating_text(pos, txt, GOLD, 1.1);
    }
    for (pos, txt) in crit_texts {
        world.add_floating_text(pos, txt, GOLD, 1.25);
    }

    // Применяем собранные эффекты по боссам, яд Директора и частицы
    for (slot, actual_dmg, dmg_type, hit_pos) in boss_hits {
        apply_damage_to_boss_slot(world, slot, actual_dmg, dmg_type, DamageSourceKind::Ranged);
        if !world.is_headless {
            world.add_floating_text(hit_pos, format!("-{:.0}", actual_dmg), RED, 1.1);
            world.spawn_particles(hit_pos, 8, RED, 70.0);
        }
    }
    for slot in director_poison_slots {
        apply_pestilence_poison_to_slot(world, slot, pestilence_lvl, 0.08, 0.50);
    }
    for (pos, hit_color) in hit_particles {
        if world.is_headless {
            break;
        }
        world.particles.push(crate::ecs::world::ParticleEntity {
            pos,
            vel: Vec2::new(
                crate::core::visual_rand::gen_range(-30.0, 30.0),
                crate::core::visual_rand::gen_range(-30.0, 30.0),
            ),
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
    for (start_pos, target_id, max_jumps, jump_rad, chain_dmg, chain_type, owner_id) in chain_lightning_events {
        let mut hit_ids = [0usize; 16];
        let mut hit_count = 1usize;
        hit_ids[0] = target_id;
        let mut curr_pos = start_pos;

        for _ in 0..max_jumps {
            world.spatial_grid.query_nearby_into(curr_pos, jump_rad, &mut nearby_scratch);
            let mut best_target = None;
            let mut min_d_sq = jump_rad * jump_rad;

            for &idx in &nearby_scratch {
                if let Some(other) = world.mobs.get(idx) {
                    if other.health.is_alive() && !hit_ids[..hit_count].contains(&other.id) {
                        let d_sq = other.pos.distance_squared(curr_pos);
                        if d_sq < min_d_sq {
                            min_d_sq = d_sq;
                            best_target = Some((other.id, other.pos, idx));
                        }
                    }
                }
            }

            if let Some((t_id, t_pos, idx)) = best_target {
                if hit_count < hit_ids.len() {
                    hit_ids[hit_count] = t_id;
                    hit_count += 1;
                }
                world.lightning_arcs.push((curr_pos, t_pos, 0.22));
                if let Some(target_mob) = world.mobs.get_mut(idx) {
                    let packet = DamagePacket::mob(
                        Some(0),
                        idx,
                        t_id,
                        chain_dmg,
                        chain_type,
                        0.8,
                    )
                    .with_owner(owner_id);
                    let dmg = target_mob.resistances.calculate_damage(&packet, target_mob.is_undead);
                    target_mob.health.take_damage(dmg);
                    if owner_id.is_some() && dmg > 0.0 {
                        target_mob.last_hit_boss_id = owner_id;
                    }
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

    // Обработка взрывов Мортиры Бедствия (CalamityMortar)
    if !mortar_explosions.is_empty() {
        world.push_audio(crate::audio::AudioEvent::FireBlast);
    }
    for (exp_pos, exp_rad, exp_dmg, owner_id) in mortar_explosions {
        world.spawn_particles(exp_pos, 16, ORANGE, exp_rad);
        world.spatial_grid.query_nearby_into(exp_pos, exp_rad, &mut nearby_scratch);
        for &idx in &nearby_scratch {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.health.is_alive() && mob.pos.distance_squared(exp_pos) < exp_rad * exp_rad {
                    let packet = DamagePacket::mob(
                        Some(0),
                        idx,
                        mob.id,
                        exp_dmg,
                        DamageType::Fire,
                        0.4,
                    )
                    .with_owner(owner_id);
                    let dmg = mob.resistances.calculate_damage(&packet, mob.is_undead);
                    mob.health.take_damage(dmg);
                    if owner_id.is_some() && dmg > 0.0 {
                        mob.last_hit_boss_id = owner_id;
                    }
                }
            }
        }
    }

    // Списываем lifetime после swept-коллизий тика и удаляем израсходованные снаряды
    for p in &mut world.projectiles {
        p.lifetime -= dt;
        if p.lifetime <= 1e-4 {
            p.lifetime = 0.0;
        }
    }
    world.projectiles.retain(|p| p.lifetime > 0.0 && p.pierce > 0);

    // 4. Смерть мобов, взрыв трупов (NetherBlaze) и дроп кристаллов душ
    let mut corpse_explosions: Vec<(Vec2, f32, f32, Option<u8>)> = Vec::new();
    let mut dropped_gems = Vec::new();
    let mut spawned_phantoms: Vec<(Vec2, Option<u8>, f32)> = Vec::new();
    let prev_mob_len = world.mobs.len();
    let max_curse_xp_mult = profiles
        .iter()
        .map(|p| p.curse_xp_mult)
        .fold(1.0f32, f32::max);

    {
        let active_boss_id = world.active_boss_id;
        let mobs = &mut world.mobs;
        let primary_inv = &mut world.boss_inventory;
        let primary_hp = &mut world.boss_health;
        let coop_bosses = &mut world.coop_bosses;

        mobs.retain(|mob| {
            if !mob.health.is_alive() {
                let killer_id = mob.last_hit_boss_id;
                let killer_slot = killer_id
                    .and_then(|id| boss_slot_by_id_parts(active_boss_id, coop_bosses, id));
                let killer_profile = killer_id
                    .and_then(|id| profiles.iter().find(|p| p.boss_id == id));

                // Пассивка "Жатва Душ" владельца последнего удара
                if let Some(kp) = killer_profile {
                    if kp.has_soul_harvester
                        && roll_slot_luck(killer_slot, 0.20, primary_inv, coop_bosses)
                    {
                        spawned_phantoms.push((mob.pos, killer_id, kp.dmg_mult));
                    }

                    // Эволюция "Пекло Преисподней": Взрыв трупов горящих врагов владельца
                    if kp.has_nether_blaze
                        && (mob.status.burn_timer > 0.0 || mob.status.burned_this_tick)
                    {
                        corpse_explosions.push((mob.pos, 55.0, 75.0, killer_id));
                    }
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

                let curse_xp_mult = killer_profile.map_or(max_curse_xp_mult, |p| p.curse_xp_mult);
                xp_value *= curse_xp_mult;

                if killer_profile.is_some_and(|p| p.has_soul_feast) {
                    heal_boss_slot_parts(killer_slot, 3.5, primary_hp, coop_bosses);
                    xp_value *= 1.25;
                }

                let martyr_lvl = primary_inv.get_passive_level(PassiveType::MartyrVanguard);
                if martyr_lvl > 0 && primary_hp.is_alive() {
                    primary_hp.heal(1.5 * martyr_lvl as f32);
                }
                for coop in coop_bosses.iter_mut() {
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
    }

    if world.mobs.len() != prev_mob_len {
        // Немедленно перестраиваем SpatialGrid после удаления мертвых мобов,
        // чтобы взрывы трупов и эгида в этом же тике использовали актуальные индексы.
        world.rebuild_spatial_grid();
    }

    // Призыв призраков от пассивки "Жатва Душ" (SoulHarvester)
    for (p_pos, owner_id, owner_dmg_mult) in spawned_phantoms {
        let angle = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
        let dir = Vec2::new(angle.cos(), angle.sin());
        let pid = world.alloc_id();
        world.projectiles.push(crate::ecs::world::ProjectileEntity {
            id: pid,
            pos: p_pos,
            prev_pos: p_pos,
            vel: dir * 320.0,
            damage: 42.0 * owner_dmg_mult,
            damage_type: DamageType::True,
            armor_penetration: 0.6,
            lifetime: 2.8,
            radius: 14.0,
            pierce: 3,
            faction: Faction::Boss,
            owner_boss_id: owner_id,
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
                if !world.is_headless {
                    world.spawn_particles(st_pos, 50, PURPLE, 190.0);
                    world.spawn_particles(st_pos, 35, RED, 150.0);
                    world.spawn_particles(st_pos, 25, GOLD, 130.0);
                    world.add_floating_text(st_pos, format!("АЛТАРЬ ТИРА {} УНИЧТОЖЕН!", altar_tier), GOLD, 1.6);
                }

                let total_altar_xp = 150.0 + altar_tier.saturating_sub(1) as f32 * 40.0;
                world.structure_contributors.remove(&st_id);
                world.grant_shared_boss_xp(total_altar_xp);
                if !world.is_headless {
                    world.add_floating_text(
                        st_pos,
                        format!("АЛТАРЬ ТИРА {} УНИЧТОЖЕН! +{:.0} XP ВСЕМ!", altar_tier, total_altar_xp),
                        GOLD,
                        1.8,
                    );
                }

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
                            if !world.is_headless {
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
                            if !world.is_headless {
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

    // Обработка взрывов трупов от NetherBlaze
    for (pos, dmg, radius, owner_boss_id) in corpse_explosions {
        world.spawn_particles(pos, 18, ORANGE, 140.0);
        world.spatial_grid.query_nearby_into(pos, radius, &mut nearby_scratch);
        for &idx in &nearby_scratch {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(pos) < radius * radius {
                    mob.health.take_damage(dmg);
                    if owner_boss_id.is_some() && dmg > 0.0 {
                        mob.last_hit_boss_id = owner_boss_id;
                    }
                }
            }
        }
    }

    world.gems.extend(dropped_gems);

    // Ограничение и слияние кристаллов душ (Gem Compression) при превышении лимита
    if world.gems.len() > 350 {
        merge_excess_gems(&mut world.gems, 300, world.boss_pos);
    }

    // 5. Подбор кристаллов душ всеми живыми Боссами за один проход
    let pickup_radius_sq = (boss_radius + 20.0).powi(2);
    let mut collected_xp = 0.0;
    let mut magnet_picked = false;
    let mut big_xp_picks: Vec<(Vec2, f32)> = Vec::new();
    let mut magnet_pick_pos: Option<Vec2> = None;

    world.gems.retain(|gem| {
        let mut picker_pos: Option<Vec2> = None;
        for profile in &profiles {
            if gem.pos.distance_squared(profile.pos) < pickup_radius_sq {
                picker_pos = Some(profile.pos);
                break;
            }
        }
        if let Some(p_pos) = picker_pos {
            if gem.is_magnet {
                magnet_picked = true;
                magnet_pick_pos.get_or_insert(p_pos);
            }
            if gem.xp_value >= 120.0 {
                big_xp_picks.push((p_pos, gem.xp_value));
            }
            collected_xp += gem.xp_value;
            false
        } else {
            true
        }
    });

    if !world.is_headless {
        for (p_pos, xp) in big_xp_picks {
            world.add_floating_text(p_pos, format!("+{:.0} XP!", xp), GOLD, 1.3);
        }
    }

    if magnet_picked {
        if let Some(m_pos) = magnet_pick_pos {
            world.add_floating_text(m_pos, "МАГНИТ АКТИВИРОВАН!".to_string(), SKYBLUE, 1.5);
        }
        for gem in &world.gems {
            collected_xp += gem.xp_value;
        }
        world.gems.clear();
    }

    if collected_xp > 0.0 {
        world.grant_shared_boss_xp(collected_xp);
    }
    crate::ecs::systems::coop::apply_all_pending_level_ups(world);

    // 6. Проверка Филактерии Возрождения (RevivalPhylactery Revive) для каждого игрока:
    let mut phylactery_bursts: Vec<(u8, Vec2)> = Vec::new();
    if !world.boss_health.is_alive()
        && world.boss_inventory.has_passive(PassiveType::RevivalPhylactery)
        && !world.boss_inventory.has_used_phylactery_revive
    {
        world.boss_inventory.has_used_phylactery_revive = true;
        world.boss_health.current = world.boss_health.max;
        world.boss_invulnerable_timer = 3.0;
        phylactery_bursts.push((world.active_boss_id, world.boss_pos));
    }
    for player in &mut world.coop_bosses {
        if !player.boss_health.is_alive()
            && player.boss_inventory.has_passive(PassiveType::RevivalPhylactery)
            && !player.boss_inventory.has_used_phylactery_revive
        {
            player.boss_inventory.has_used_phylactery_revive = true;
            player.boss_health.current = player.boss_health.max;
            player.boss_invulnerable_timer = 3.0;
            phylactery_bursts.push((player.id, player.boss_pos));
        }
    }
    for (boss_id, rev_pos) in phylactery_bursts {
        world.add_floating_text(
            rev_pos,
            "ВОСКРЕШЕНИЕ: ФИЛАКТЕРИЯ ВОЗРОЖДЕНИЯ!".to_string(),
            GOLD,
            1.8,
        );
        world.push_audio(crate::audio::AudioEvent::BossLevelUp);
        world.screen_shake = 18.0;
        world.spawn_particles(rev_pos, 60, GOLD, 260.0);
        world.spawn_particles(rev_pos, 40, WHITE, 320.0);
        apply_radial_mob_burst(world, rev_pos, 320.0, 160.0, 480.0, Some(boss_id));
    }

    // 7. Гарантируем актуальность индексов SpatialGrid к началу следующего тика.
    if world.spatial_grid.len() != world.mobs.len() {
        world.rebuild_spatial_grid();
    }
}

/// Один проход слияния кристаллов душ в ячейках `cell x cell` с сохранением нулевого и первого моментов:
/// M0 = sum(xp_i), центр масс = sum(xp_i * pos_i) / M0.
fn compress_gems_grid(gems: &mut Vec<SoulGemEntity>, cell: f32, boss_pos: Vec2) {
    let boss_dist_min_sq = 150.0 * 150.0;
    let inv_cell = 1.0 / cell.max(1.0);
    let mut keys: Vec<(u64, usize)> = Vec::with_capacity(gems.len());

    for (i, g) in gems.iter().enumerate() {
        if g.is_magnet || g.xp_value <= 0.0 || g.pos.distance_squared(boss_pos) < boss_dist_min_sq {
            continue;
        }
        let cx = (g.pos.x.max(0.0) * inv_cell).floor() as u32;
        let cy = (g.pos.y.max(0.0) * inv_cell).floor() as u32;
        keys.push((((cx as u64) << 32) | (cy as u64), i));
    }

    keys.sort_unstable_by_key(|&(k, _)| k);

    let mut idx = 0;
    while idx < keys.len() {
        let (cell_key, rep) = keys[idx];
        let mut m0 = gems[rep].xp_value;
        let mut m1 = gems[rep].pos * m0;
        idx += 1;

        while idx < keys.len() && keys[idx].0 == cell_key {
            let i = keys[idx].1;
            let xp = gems[i].xp_value;
            m0 += xp;
            m1 += gems[i].pos * xp;
            gems[i].xp_value = 0.0;
            idx += 1;
        }

        if m0 > 0.0 {
            gems[rep].pos = m1 / m0;
            gems[rep].xp_value = m0;
        }
    }

    gems.retain(|g| g.is_magnet || g.xp_value > 0.0);
}

/// Слияние избыточных кристаллов душ (Gem Merging):
/// Предотвращает просадки FPS от тысяч объектов на карте при сохранении 100% полученного опыта и центра масс!
fn merge_excess_gems(gems: &mut Vec<SoulGemEntity>, target_max: usize, boss_pos: Vec2) {
    if gems.len() <= target_max {
        return;
    }
    compress_gems_grid(gems, 70.0, boss_pos);
    if gems.len() > target_max {
        compress_gems_grid(gems, 140.0, boss_pos);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::world::MobEntity;

    #[test]
    fn test_damage_queue_direct_index_fallback_and_capacity_retention() {
        let mut world = World::new();
        world.is_headless = true;
        world.damage_queue.reserve(128);
        let initial_cap = world.damage_queue.capacity();

        world.mobs.push(MobEntity::new_test(101, Vec2::new(100.0, 100.0), 1));
        world.mobs.push(MobEntity::new_test(102, Vec2::new(200.0, 200.0), 1));
        world.rebuild_spatial_grid();

        // Индекс 0 точный, индекс 99 устаревший (fallback по id = 102)
        world.damage_queue.push(DamagePacket::mob(Some(0), 0, 101, 25.0, DamageType::True, 1.0));
        world.damage_queue.push(DamagePacket::mob(Some(0), 99, 102, 40.0, DamageType::True, 1.0));
        // Индекс 0 указывает на 101, но id = 102 -> тоже fallback по id
        world.damage_queue.push(DamagePacket::mob(Some(0), 0, 102, 10.0, DamageType::True, 1.0));

        update_combat(&mut world, 0.05);

        assert!((world.mobs[0].health.current - 75.0).abs() < 1e-3);
        assert!((world.mobs[1].health.current - 50.0).abs() < 1e-3);
        assert!(world.damage_queue.is_empty());
        assert!(world.damage_queue.capacity() >= initial_cap);
    }
}
