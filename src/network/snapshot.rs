use macroquad::prelude::Vec2;
use serde::{Deserialize, Serialize};

use crate::ecs::components::combat::{DamageType, Faction, Health, Resistances, StatusEffects};
use crate::ecs::components::inventory::{BossInventory, WeaponType};
use crate::ecs::world::{
    AcidPoolEntity, BossPlayerState, MobEntity, ProjectileEntity, SoulGemEntity,
    StructureEntity, World,
};
use crate::protocol::commands::NetVec2;

#[inline]
fn min_dist_sq_to_focus(pos: Vec2, focus: &[Vec2]) -> f32 {
    let mut best = f32::MAX;
    for &pt in focus {
        let d = pt.distance_squared(pos);
        if d < best {
            best = d;
        }
    }
    best
}

/// Выбирает `count` ближайших к набору точек `focus` индексов без полной O(N log N) сортировки:
/// ровно 1 линейный проход расчёта дистанций в кэш-локальный буфер + `select_nth_unstable_by` за O(N).
fn nearest_indices<T>(items: &[T], count: usize, focus: &[Vec2], position: impl Fn(&T) -> Vec2) -> Vec<usize> {
    if count == 0 || items.is_empty() {
        return Vec::new();
    }
    if items.len() <= count {
        return (0..items.len()).collect();
    }

    let mut scored: Vec<(f32, usize)> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| (min_dist_sq_to_focus(position(item), focus), idx))
        .collect();

    scored.select_nth_unstable_by(count - 1, |a, b| a.0.total_cmp(&b.0));
    scored[..count].iter().map(|&(_, idx)| idx).collect()
}

/// Адаптивный выбор Top-K ближайших мобов:
/// - Если боссы расположены кучно (разброс <= 2400 px) и мобов много (> 1500),
///   использует Multi-Source Ring BFS по готовой `SpatialGrid` и останавливается,
///   как только доказано, что вне кольца не осталось более близких целей (0 обходов дальних 18к мобов).
/// - Если боссы разбросаны по всей карте или мобов мало — мгновенно переходит на O(N) `select_nth_unstable_by`.
fn nearest_mob_indices(world: &World, count: usize, focus: &[Vec2]) -> Vec<usize> {
    let n = world.mobs.len();
    if count == 0 || n == 0 {
        return Vec::new();
    }
    if n <= count {
        return (0..n).collect();
    }

    if n > 1500 && !focus.is_empty() && world.spatial_grid.len() == n {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &pt in focus {
            min_x = min_x.min(pt.x);
            max_x = max_x.max(pt.x);
            min_y = min_y.min(pt.y);
            max_y = max_y.max(pt.y);
        }
        let focus_span = (max_x - min_x).max(max_y - min_y);

        if focus_span <= 2400.0 {
            let grid = &world.spatial_grid;
            let cols = grid.cols();
            let rows = grid.rows();
            let cell_size = grid.cell_size();

            let mut focus_cells = Vec::with_capacity(focus.len());
            let mut all_valid = true;
            for &pt in focus {
                if let Some(c) = grid.get_cell_coords(pt) {
                    focus_cells.push(c);
                } else {
                    all_valid = false;
                    break;
                }
            }

            if all_valid && cols > 0 && rows > 0 {
                let mut visited = grid.new_visited_mask();
                let mut candidates: Vec<(f32, usize)> = Vec::with_capacity(count * 2);
                let max_ring = 22usize;

                for ring in 0..=max_ring {
                    if ring == 0 {
                        for &(fcx, fcy) in &focus_cells {
                            grid.visit_cell_with_mask(fcx, fcy, &mut visited, &mut |idx| {
                                let d_sq = min_dist_sq_to_focus(world.mobs[idx].pos, focus);
                                candidates.push((d_sq, idx));
                            });
                        }
                    } else {
                        for &(fcx, fcy) in &focus_cells {
                            let fx0 = fcx.saturating_sub(ring);
                            let fx1 = (fcx + ring).min(cols - 1);
                            let fy0 = fcy.saturating_sub(ring);
                            let fy1 = (fcy + ring).min(rows - 1);

                            if fcx >= ring {
                                let nx = fcx - ring;
                                for ny in fy0..=fy1 {
                                    grid.visit_cell_with_mask(nx, ny, &mut visited, &mut |idx| {
                                        candidates.push((min_dist_sq_to_focus(world.mobs[idx].pos, focus), idx));
                                    });
                                }
                            }
                            if fcx + ring < cols {
                                let nx = fcx + ring;
                                for ny in fy0..=fy1 {
                                    grid.visit_cell_with_mask(nx, ny, &mut visited, &mut |idx| {
                                        candidates.push((min_dist_sq_to_focus(world.mobs[idx].pos, focus), idx));
                                    });
                                }
                            }
                            if fcy >= ring {
                                let ny = fcy - ring;
                                for nx in fx0..=fx1 {
                                    grid.visit_cell_with_mask(nx, ny, &mut visited, &mut |idx| {
                                        candidates.push((min_dist_sq_to_focus(world.mobs[idx].pos, focus), idx));
                                    });
                                }
                            }
                            if fcy + ring < rows {
                                let ny = fcy + ring;
                                for nx in fx0..=fx1 {
                                    grid.visit_cell_with_mask(nx, ny, &mut visited, &mut |idx| {
                                        candidates.push((min_dist_sq_to_focus(world.mobs[idx].pos, focus), idx));
                                    });
                                }
                            }
                        }
                    }

                    if ring >= 2 && candidates.len() >= count {
                        let safe_radius = ((ring - 1) as f32) * cell_size;
                        let safe_sq = safe_radius * safe_radius;
                        let confirmed = candidates.iter().filter(|&&(d_sq, _)| d_sq <= safe_sq).count();
                        if confirmed >= count {
                            candidates.select_nth_unstable_by(count - 1, |a, b| a.0.total_cmp(&b.0));
                            return candidates[..count].iter().map(|&(_, idx)| idx).collect();
                        }
                    }
                }
            }
        }
    }

    nearest_indices(&world.mobs, count, focus, |mob| mob.pos)
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
    #[serde(alias = "blade_storm_angle")]
    pub blade_squall_angle: f32,
    #[serde(alias = "divine_shield_angle")]
    pub light_bastion_angle: f32,
    #[serde(default)]
    pub level_up_options: Option<Vec<crate::ecs::components::inventory::PerkChoice>>,
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
    #[serde(default)]
    pub is_hero: bool,
    #[serde(default)]
    pub hero_enrage_timer: f32,
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
pub struct BlackHoleSnapshot {
    pub id: usize,
    pub owner_id: usize,
    pub center_pos: NetVec2,
    pub radius: f32,
    pub orbit_angle: f32,
    pub orbit_dist: f32,
    pub orbit_speed: f32,
    pub duration: f32,
    pub is_active: bool,
    pub spin_angle: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub sequence: u32,
    pub game_time: f32,
    #[serde(default = "default_match_duration_secs")]
    pub match_duration_secs: f32,
    #[serde(default)]
    pub is_paused: bool,
    #[serde(default)]
    pub pause_initiator: Option<String>,
    pub bosses: Vec<BossSnapshot>,
    pub director: Option<crate::ecs::world::DirectorState>,
    pub mobs: Vec<MobSnapshot>,
    pub projectiles: Vec<ProjectileSnapshot>,
    pub gems: Vec<GemSnapshot>,
    pub structures: Vec<StructureSnapshot>,
    pub acid_pools: Vec<AcidSnapshot>,
    #[serde(default)]
    pub black_holes: Vec<BlackHoleSnapshot>,
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
            blade_squall_angle: world.blade_squall_angle,
            light_bastion_angle: world.light_bastion_angle,
            level_up_options: world.boss_level_up_options.clone(),
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
            blade_squall_angle: player.blade_squall_angle,
            light_bastion_angle: player.light_bastion_angle,
            level_up_options: player.level_up_options.clone(),
        }));

        let mut focus: Vec<Vec2> = bosses.iter()
            .filter(|boss| boss.health.is_alive())
            .map(|boss| boss.pos.into())
            .collect();
        if focus.is_empty() { focus.push(world.boss_pos); }
        let mob_indices = nearest_mob_indices(world, 600, &focus);
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
                is_hero: mob.is_hero, hero_enrage_timer: mob.hero_enrage_timer,
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
            black_holes: world.black_holes.iter().map(|bh| BlackHoleSnapshot {
                id: bh.id, owner_id: bh.owner_id, center_pos: bh.center_pos.into(),
                radius: bh.radius, orbit_angle: bh.orbit_angle, orbit_dist: bh.orbit_dist,
                orbit_speed: bh.orbit_speed, duration: bh.duration, is_active: bh.is_active,
                spin_angle: bh.spin_angle,
            }).collect(),
            is_paused: world.is_network_paused,
            pause_initiator: world.pause_initiator.clone(),
        }
    }
}

/// Replaces the client's presentation state with the latest authoritative host snapshot.
/// The local player's actor is mapped into the legacy primary Boss fields so existing HUD
/// panels continue to show that player's inventory and health.
pub fn apply_to_world(snapshot: WorldSnapshot, world: &mut World, local_player_id: u8) {
    world.game_time = snapshot.game_time;
    world.match_duration_secs = snapshot.match_duration_secs;
    world.is_network_paused = snapshot.is_paused;
    world.pause_initiator = snapshot.pause_initiator;
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
        world.blade_squall_angle = local.blade_squall_angle;
        world.light_bastion_angle = local.light_bastion_angle;
        world.boss_level_up_options = local.level_up_options.clone();
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
            player.blade_squall_angle = boss.blade_squall_angle;
            player.light_bastion_angle = boss.light_bastion_angle;
            player.level_up_options = boss.level_up_options.clone();
            player
        }).collect();

    world.mobs = snapshot.mobs.into_iter().map(|mob| MobEntity {
        id: mob.id, target_boss_id: mob.target_boss_id, pos: mob.pos.into(), vel: mob.vel.into(), speed: 0.0,
        health: mob.health, resistances: Resistances::default(), status: mob.status,
        damage: 0.0, tier: mob.tier, radius: mob.radius, is_undead: mob.is_undead,
        is_elite: mob.is_elite, knockback_immune: false, knockback_vel: Vec2::ZERO,
        attack_timer: 0.0,
        is_hero: mob.is_hero,
        hero_enrage_timer: mob.hero_enrage_timer,
        hero_skill_timers: [0.0; 4],
        attack_count: 0,
        has_duplicated: false,
        owner_hero_id: None,
        last_hit_boss_id: None,
    }).collect();
    world.projectiles = snapshot.projectiles.into_iter().map(|projectile| ProjectileEntity {
        id: projectile.id, pos: projectile.pos.into(), prev_pos: projectile.pos.into(), vel: projectile.vel.into(),
        damage: projectile.damage, damage_type: projectile.damage_type, armor_penetration: 0.0,
        lifetime: projectile.lifetime, radius: projectile.radius, pierce: 0,
        faction: projectile.faction, owner_boss_id: None, source_weapon: projectile.source_weapon, homing_strength: 0.0,
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
        corrosive: pool.corrosive, is_director_source: pool.is_director_source, owner_boss_id: None,
    }).collect();
    world.black_holes = snapshot.black_holes.into_iter().map(|bh| crate::ecs::world::BlackHoleEntity {
        id: bh.id,
        owner_id: bh.owner_id,
        center_pos: bh.center_pos.into(),
        radius: bh.radius,
        orbit_angle: bh.orbit_angle,
        orbit_dist: bh.orbit_dist,
        orbit_speed: bh.orbit_speed,
        duration: bh.duration,
        is_active: bh.is_active,
        spin_angle: bh.spin_angle,
    }).collect();
    world.rebuild_spatial_grid();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_snapshot_pause_synchronization() {
        let mut world = World::new();
        world.is_network_paused = true;
        world.pause_initiator = Some("Босс (Игрок)".to_string());

        let snapshot = WorldSnapshot::from_world(&world, 42);
        assert!(snapshot.is_paused);
        assert_eq!(snapshot.pause_initiator.as_deref(), Some("Босс (Игрок)"));

        let json = serde_json::to_string(&snapshot).expect("Serialize snapshot");
        let decoded: WorldSnapshot = serde_json::from_str(&json).expect("Deserialize snapshot");
        assert!(decoded.is_paused);
        assert_eq!(decoded.pause_initiator.as_deref(), Some("Босс (Игрок)"));

        let mut client_world = World::new();
        assert!(!client_world.is_network_paused);
        apply_to_world(decoded, &mut client_world, 1);
        assert!(client_world.is_network_paused);
        assert_eq!(client_world.pause_initiator.as_deref(), Some("Босс (Игрок)"));
    }

    #[test]
    fn test_apply_to_world_rebuilds_spatial_grid() {
        let mut host_world = World::new();
        let mob_pos = Vec2::new(1200.0, 900.0);
        host_world.mobs.push(MobEntity::new_test(501, mob_pos, 1));
        host_world.rebuild_spatial_grid();

        let snapshot = WorldSnapshot::from_world(&host_world, 7);
        let mut client_world = World::new();
        apply_to_world(snapshot, &mut client_world, 1);

        assert_eq!(client_world.mobs.len(), 1);
        assert_eq!(client_world.spatial_grid.len(), 1);
        let nearby = client_world.spatial_grid.query_nearby(mob_pos, 100.0);
        assert_eq!(nearby, vec![0]);
    }

    #[test]
    fn test_snapshot_nearest_topk_matches_exact_ground_truth() {
        let mut world = World::new();
        let boss1 = Vec2::new(5000.0, 5000.0);
        let boss2_near = Vec2::new(5400.0, 5100.0); // Clustered (span = 400 <= 2400)
        world.boss_pos = boss1;

        // Создаем 2500 мобов по всей карте
        for i in 0..2500usize {
            let angle = (i as f32) * 0.37;
            let radius = 30.0 + ((i % 900) as f32) * 5.0;
            let pos = boss1 + Vec2::new(angle.cos() * radius, angle.sin() * radius);
            let clamped = Vec2::new(
                pos.x.clamp(50.0, crate::core::MAP_WIDTH - 50.0),
                pos.y.clamp(50.0, crate::core::MAP_HEIGHT - 50.0),
            );
            let id = world.alloc_id();
            world.mobs.push(MobEntity::new_test(id, clamped, 1));
        }
        world.rebuild_spatial_grid();

        let focus_clustered = [boss1, boss2_near];
        let fast_indices = nearest_mob_indices(&world, 600, &focus_clustered);
        assert_eq!(fast_indices.len(), 600);

        // Проверяем точность против полной эталонной сортировки
        let mut exact: Vec<(f32, usize)> = world
            .mobs
            .iter()
            .enumerate()
            .map(|(i, m)| (min_dist_sq_to_focus(m.pos, &focus_clustered), i))
            .collect();
        exact.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

        let cutoff_dist_sq = exact[599].0;
        let max_fast_dist_sq = fast_indices
            .iter()
            .map(|&i| min_dist_sq_to_focus(world.mobs[i].pos, &focus_clustered))
            .fold(0.0f32, f32::max);
        assert!(
            (max_fast_dist_sq - cutoff_dist_sq).abs() < 1e-3,
            "Ring-BFS Top-600 cutoff mismatch: fast={}, exact={}",
            max_fast_dist_sq,
            cutoff_dist_sq
        );

        // Тест разнесённых боссов (fallback на buffer + select_nth)
        let boss2_far = Vec2::new(15000.0, 12000.0);
        let focus_spread = [boss1, boss2_far];
        let spread_indices = nearest_mob_indices(&world, 600, &focus_spread);
        assert_eq!(spread_indices.len(), 600);

        let mut exact_spread: Vec<f32> = world
            .mobs
            .iter()
            .map(|m| min_dist_sq_to_focus(m.pos, &focus_spread))
            .collect();
        exact_spread.sort_unstable_by(|a, b| a.total_cmp(b));

        let spread_cutoff = exact_spread[599];
        let max_spread_dist = spread_indices
            .iter()
            .map(|&i| min_dist_sq_to_focus(world.mobs[i].pos, &focus_spread))
            .fold(0.0f32, f32::max);
        assert!(
            (max_spread_dist - spread_cutoff).abs() < 1e-3,
            "Spread Top-600 cutoff mismatch"
        );
    }
}
