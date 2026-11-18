use macroquad::prelude::Vec2;
use serde::{Deserialize, Serialize};

use crate::ecs::components::combat::{DamageType, Faction, Health, Resistances, StatusEffects};
use crate::ecs::components::inventory::{BossInventory, WeaponType};
use crate::ecs::world::{
    AcidPoolEntity, BossPlayerState, MobEntity, ProjectileEntity, SoulGemEntity,
    StructureEntity, World,
};
use crate::protocol::commands::NetVec2;

fn nearest_indices<T>(items: &[T], count: usize, focus: &[Vec2], position: impl Fn(&T) -> Vec2) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..items.len()).collect();
    indices.sort_unstable_by(|&a, &b| {
        let distance = |item: &T| {
            let pos = position(item);
            focus.iter().map(|point| point.distance_squared(pos)).fold(f32::MAX, f32::min)
        };
        distance(&items[a]).total_cmp(&distance(&items[b]))
    });
    indices.truncate(count);
    indices
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BossSnapshot {
    pub id: u8,
    pub pos: NetVec2,
    pub vel: NetVec2,
    pub facing: NetVec2,
    pub name: String,
    pub skin: u8,
    pub health: Health,
    pub resistances: Resistances,
    pub inventory: BossInventory,
    pub invulnerable_timer: f32,
    pub poison_timer: f32,
    pub poison_dps: f32,
    pub still_time: f32,
    pub mushroom_still_time: f32,
    pub revive_progress: f32,
    pub dash_cooldown: f32,
    pub blade_storm_angle: f32,
    pub divine_shield_angle: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MobSnapshot {
    pub id: usize,
    pub target_boss_id: Option<u8>,
    pub pos: NetVec2,
    pub vel: NetVec2,
    pub health: Health,
    pub status: StatusEffects,
    pub tier: u8,
    pub radius: f32,
    pub is_undead: bool,
    pub is_elite: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectileSnapshot {
    pub id: usize,
    pub pos: NetVec2,
    pub vel: NetVec2,
    pub damage: f32,
    pub damage_type: DamageType,
    pub radius: f32,
    pub faction: Faction,
    pub source_weapon: Option<WeaponType>,
    pub lifetime: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GemSnapshot {
    pub id: usize,
    pub pos: NetVec2,
    pub xp_value: f32,
    pub is_magnet: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StructureSnapshot {
    pub id: usize,
    pub pos: NetVec2,
    pub health: Health,
    pub resistances: Resistances,
    pub faction: Faction,
    pub radius: f32,
    pub structure_type: u8,
    pub pulse_timer: f32,
    pub spawn_timer: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub is_active: bool,
    pub state_timer: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AcidSnapshot {
    pub pos: NetVec2,
    pub radius: f32,
    pub lifetime: f32,
    pub dps: f32,
    pub corrosive: bool,
    pub is_director_source: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub sequence: u32,
    pub game_time: f32,
    #[serde(default = "default_match_duration_secs")]
    pub match_duration_secs: f32,
    pub bosses: Vec<BossSnapshot>,
    pub director: Option<crate::ecs::world::DirectorState>,
    pub mobs: Vec<MobSnapshot>,
    pub projectiles: Vec<ProjectileSnapshot>,
    pub gems: Vec<GemSnapshot>,
    pub structures: Vec<StructureSnapshot>,
    pub acid_pools: Vec<AcidSnapshot>,
}

fn default_match_duration_secs() -> f32 {
    crate::core::time::GAME_DURATION_SECS
}

impl WorldSnapshot {
    pub fn from_world(world: &World, sequence: u32) -> Self {
        let mut bosses = Vec::with_capacity(world.coop_bosses.len() + 1);
        bosses.push(BossSnapshot {
            id: 1,
            pos: world.boss_pos.into(),
            vel: world.boss_vel.into(),
            facing: world.boss_facing.into(),
            name: world.boss_name.clone(),
            skin: world.boss_skin,
            health: world.boss_health.clone(),
            resistances: world.boss_resistances.clone(),
            inventory: world.boss_inventory.clone(),
            invulnerable_timer: world.boss_invulnerable_timer,
            poison_timer: world.boss_poison_timer,
            poison_dps: world.boss_poison_dps,
            still_time: world.boss_still_time,
            mushroom_still_time: world.boss_mushroom_still_time,
            revive_progress: world.boss_revive_progress,
            dash_cooldown: world.boss_dash_cooldown,
            blade_storm_angle: world.blade_storm_angle,
            divine_shield_angle: world.divine_shield_angle,
        });
        bosses.extend(world.coop_bosses.iter().map(|player| BossSnapshot {
            id: player.id,
            pos: player.boss_pos.into(),
            vel: player.boss_vel.into(),
            facing: player.boss_facing.into(),
            name: player.boss_name.clone(),
            skin: player.boss_skin,
            health: player.boss_health.clone(),
            resistances: player.boss_resistances.clone(),
            inventory: player.boss_inventory.clone(),
            invulnerable_timer: player.boss_invulnerable_timer,
            poison_timer: player.boss_poison_timer,
            poison_dps: player.boss_poison_dps,
            still_time: player.boss_still_time,
            mushroom_still_time: player.boss_mushroom_still_time,
            revive_progress: player.revive_progress,
            dash_cooldown: player.boss_dash_cooldown,
            blade_storm_angle: player.blade_storm_angle,
            divine_shield_angle: player.divine_shield_angle,
        }));

        let mut focus: Vec<Vec2> = bosses.iter()
            .filter(|boss| boss.health.is_alive())
            .map(|boss| boss.pos.into())
            .collect();
        if focus.is_empty() { focus.push(world.boss_pos); }
        let mob_indices = nearest_indices(&world.mobs, 600, &focus, |mob| mob.pos);
        let projectile_indices = nearest_indices(&world.projectiles, 320, &focus, |projectile| projectile.pos);
        let gem_indices = nearest_indices(&world.gems, 350, &focus, |gem| gem.pos);
        let acid_indices = nearest_indices(&world.acid_pools, 128, &focus, |pool| pool.pos);

        Self {
            sequence,
            game_time: world.game_time,
            match_duration_secs: world.match_duration_secs,
            bosses,
            director: world.directors.first().cloned(),
            mobs: mob_indices.into_iter().map(|index| &world.mobs[index]).map(|mob| MobSnapshot {
                id: mob.id, target_boss_id: mob.target_boss_id, pos: mob.pos.into(), vel: mob.vel.into(), health: mob.health.clone(),
                status: mob.status.clone(), tier: mob.tier, radius: mob.radius,
                is_undead: mob.is_undead, is_elite: mob.is_elite,
            }).collect(),
            projectiles: projectile_indices.into_iter().map(|index| &world.projectiles[index]).map(|projectile| ProjectileSnapshot {
                id: projectile.id, pos: projectile.pos.into(), vel: projectile.vel.into(),
                damage: projectile.damage, damage_type: projectile.damage_type,
                radius: projectile.radius, faction: projectile.faction,
                source_weapon: projectile.source_weapon, lifetime: projectile.lifetime,
            }).collect(),
            gems: gem_indices.into_iter().map(|index| &world.gems[index]).map(|gem| GemSnapshot {
                id: gem.id, pos: gem.pos.into(), xp_value: gem.xp_value, is_magnet: gem.is_magnet,
            }).collect(),
            structures: world.structures.iter().map(|structure| StructureSnapshot {
                id: structure.id, pos: structure.pos.into(), health: structure.health.clone(),
                resistances: structure.resistances.clone(), faction: structure.faction,
                radius: structure.radius, structure_type: structure.structure_type,
                pulse_timer: structure.pulse_timer, spawn_timer: structure.spawn_timer,
                lifetime: structure.lifetime, max_lifetime: structure.max_lifetime,
                is_active: structure.is_active, state_timer: structure.state_timer,
            }).collect(),
            acid_pools: acid_indices.into_iter().map(|index| &world.acid_pools[index]).map(|pool| AcidSnapshot {
                pos: pool.pos.into(), radius: pool.radius, lifetime: pool.lifetime,
                dps: pool.dps, corrosive: pool.corrosive, is_director_source: pool.is_director_source,
            }).collect(),
        }
    }
}

/// Replaces the client's presentation state with the latest authoritative host snapshot.
/// The local player's actor is mapped into the legacy primary Boss fields so existing HUD
/// panels continue to show that player's inventory and health.
pub fn apply_to_world(snapshot: WorldSnapshot, world: &mut World, local_player_id: u8) {
    world.game_time = snapshot.game_time;
    world.match_duration_secs = snapshot.match_duration_secs;
    world.directors.clear();
    if let Some(director) = snapshot.director { world.directors.push(director); }

    let local_index = snapshot.bosses.iter().position(|boss| boss.id == local_player_id)
        .or_else(|| (!snapshot.bosses.is_empty()).then_some(0));
    if let Some(index) = local_index {
        let local = &snapshot.bosses[index];
        world.active_boss_id = local.id;
        world.boss_pos = local.pos.into();
        world.boss_vel = local.vel.into();
        world.boss_facing = local.facing.into();
        world.boss_name.clone_from(&local.name);
        world.boss_skin = local.skin;
        world.boss_health = local.health.clone();
        world.boss_resistances = local.resistances.clone();
        world.boss_inventory = local.inventory.clone();
        world.boss_invulnerable_timer = local.invulnerable_timer;
        world.boss_poison_timer = local.poison_timer;
        world.boss_poison_dps = local.poison_dps;
        world.boss_still_time = local.still_time;
        world.boss_mushroom_still_time = local.mushroom_still_time;
        world.boss_revive_progress = local.revive_progress;
        world.boss_dash_cooldown = local.dash_cooldown;
        world.blade_storm_angle = local.blade_storm_angle;
        world.divine_shield_angle = local.divine_shield_angle;
    }

    world.coop_bosses = snapshot.bosses.iter().enumerate()
        .filter(|(index, _)| Some(*index) != local_index)
        .map(|(_, boss)| {
            let mut player = BossPlayerState::new(boss.id, boss.pos.into(), boss.name.clone(), boss.skin);
            player.boss_vel = boss.vel.into();
            player.boss_facing = boss.facing.into();
            player.boss_health = boss.health.clone();
            player.boss_resistances = boss.resistances.clone();
            player.boss_inventory = boss.inventory.clone();
            player.boss_invulnerable_timer = boss.invulnerable_timer;
            player.boss_poison_timer = boss.poison_timer;
            player.boss_poison_dps = boss.poison_dps;
            player.boss_still_time = boss.still_time;
            player.boss_mushroom_still_time = boss.mushroom_still_time;
            player.revive_progress = boss.revive_progress;
            player.boss_dash_cooldown = boss.dash_cooldown;
            player.blade_storm_angle = boss.blade_storm_angle;
            player.divine_shield_angle = boss.divine_shield_angle;
            player
        }).collect();

    world.mobs = snapshot.mobs.into_iter().map(|mob| MobEntity {
        id: mob.id, target_boss_id: mob.target_boss_id, pos: mob.pos.into(), vel: mob.vel.into(), speed: 0.0,
        health: mob.health, resistances: Resistances::default(), status: mob.status,
        damage: 0.0, tier: mob.tier, radius: mob.radius, is_undead: mob.is_undead,
        is_elite: mob.is_elite, knockback_immune: false, knockback_vel: Vec2::ZERO,
        attack_timer: 0.0,
    }).collect();
    world.projectiles = snapshot.projectiles.into_iter().map(|projectile| ProjectileEntity {
        id: projectile.id, pos: projectile.pos.into(), vel: projectile.vel.into(),
        damage: projectile.damage, damage_type: projectile.damage_type, armor_penetration: 0.0,
        lifetime: projectile.lifetime, radius: projectile.radius, pierce: 0,
        faction: projectile.faction, source_weapon: projectile.source_weapon, homing_strength: 0.0,
    }).collect();
    world.gems = snapshot.gems.into_iter().map(|gem| SoulGemEntity {
        id: gem.id, pos: gem.pos.into(), xp_value: gem.xp_value, is_magnet: gem.is_magnet,
    }).collect();
    world.structures = snapshot.structures.into_iter().map(|structure| StructureEntity {
        id: structure.id, pos: structure.pos.into(), health: structure.health,
        resistances: structure.resistances, faction: structure.faction, radius: structure.radius,
        structure_type: structure.structure_type, pulse_timer: structure.pulse_timer,
        spawn_timer: structure.spawn_timer, lifetime: structure.lifetime,
        max_lifetime: structure.max_lifetime, is_active: structure.is_active,
        state_timer: structure.state_timer,
    }).collect();
    world.acid_pools = snapshot.acid_pools.into_iter().map(|pool| AcidPoolEntity {
        pos: pool.pos.into(), radius: pool.radius, lifetime: pool.lifetime, dps: pool.dps,
        corrosive: pool.corrosive, is_director_source: pool.is_director_source,
    }).collect();
}
