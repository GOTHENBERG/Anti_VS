use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub const STRUCTURE_ALTAR: u8 = 1;
pub const STRUCTURE_WALL: u8 = 2;
pub const STRUCTURE_OBELISK: u8 = 3;
pub const STRUCTURE_TIER_ALTAR_BASE: u8 = 20;
pub const MAX_ALTAR_TIER: u8 = 16;

pub const BLOOD_ALTAR_COST: f32 = 50.0;
pub const ALTAR_MIN_SPACING: f32 = 120.0;
pub const BUILD_GRID_SIZE: f32 = 50.0;
pub const ALTAR_BASE_COOLDOWN_SECONDS: f32 = 45.0;

pub const WALL_COST: f32 = 10.0;
pub const WALL_MAX_COOLDOWN: f32 = 2.0;
pub const WALL_LIFETIME: f32 = 60.0;
pub const WALL_RADIUS: f32 = 25.0; // 50x50 px баррикада
pub const OBELISK_RADIUS: f32 = 75.0; // 300x300 px визуальный размер
pub const OBELISK_FOOTPRINT_HALF_EXTENT: f32 = OBELISK_RADIUS * 2.0;
pub const OBELISK_BASE_HP_PER_BOSS: f32 = 2400.0; // 2400 HP на каждого живого босса
pub const OBELISK_CHARGE_SECONDS: f32 = 180.0; // 3 минуты до инфернального прорыва
pub const OBELISK_REWARD_BASE_XP: f32 = 350.0;
pub const OBELISK_REWARD_NEXT_LEVEL_PERCENT: f32 = 0.25; // +25% от шкалы опыта следующего уровня

pub fn structure_type_for_altar_tier(tier: u8) -> u8 {
    let tier = tier.clamp(1, MAX_ALTAR_TIER);
    if tier == 1 { STRUCTURE_ALTAR } else { STRUCTURE_TIER_ALTAR_BASE + tier }
}

pub fn altar_tier_from_structure_type(structure_type: u8) -> Option<u8> {
    if structure_type == STRUCTURE_ALTAR {
        Some(1)
    } else if (STRUCTURE_TIER_ALTAR_BASE + 2..=STRUCTURE_TIER_ALTAR_BASE + MAX_ALTAR_TIER)
        .contains(&structure_type)
    {
        Some(structure_type - STRUCTURE_TIER_ALTAR_BASE)
    } else {
        None
    }
}

pub fn is_altar_structure_type(structure_type: u8) -> bool {
    altar_tier_from_structure_type(structure_type).is_some()
}

pub fn altar_cost(tier: u8) -> f32 {
    if tier <= 1 {
        BLOOD_ALTAR_COST
    } else {
        crate::content::mobs::get_squad_config(tier).cost * 1.6
    }
}

pub fn altar_cooldown(tier: u8) -> f32 {
    ALTAR_BASE_COOLDOWN_SECONDS + tier.saturating_sub(1) as f32 * 1.5
}

pub fn altar_spawn_interval(tier: u8) -> f32 {
    1.0 + tier.saturating_sub(1) as f32 * 0.15
}

/// Converts a world coordinate to the center of its 50 px placement cell.
pub fn snap_to_build_grid(pos: Vec2) -> Vec2 {
    let snap_axis = |value: f32, extent: f32| {
        ((value / BUILD_GRID_SIZE).floor() * BUILD_GRID_SIZE + BUILD_GRID_SIZE * 0.5)
            .clamp(BUILD_GRID_SIZE * 0.5, extent - BUILD_GRID_SIZE * 0.5)
    };
    Vec2::new(snap_axis(pos.x, crate::core::MAP_WIDTH), snap_axis(pos.y, crate::core::MAP_HEIGHT))
}

fn apply_carapace(resistances: &mut Resistances, level: u32) {
    resistances.flat_armor += level as f32 * 2.5;
    resistances.physical_res = (resistances.physical_res + level as f32 * 0.05).min(0.70);
}

/// Applies a newly purchased Carapace level to already placed Director structures.
pub fn upgrade_structure_carapace(world: &mut World, levels: u32) {
    if levels == 0 {
        return;
    }
    for structure in &mut world.structures {
        if structure.faction == Faction::Director {
            apply_carapace(&mut structure.resistances, levels);
        }
    }
}

fn distance_sq_to_wall_aabb(point: Vec2, center: Vec2) -> f32 {
    let dx = (point.x - center.x).abs() - WALL_RADIUS;
    let dy = (point.y - center.y).abs() - WALL_RADIUS;
    dx.max(0.0).powi(2) + dy.max(0.0).powi(2)
}

/// Sleeping obelisks are invulnerable to Boss damage until their rift activates.
pub fn boss_can_damage_structure(structure: &StructureEntity) -> bool {
    structure.structure_type != STRUCTURE_OBELISK || structure.is_active
}

/// Проверяет, можно ли поставить алтарь в запрошенной точке.
pub fn can_place_blood_altar(world: &World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    let margin = 45.0;
    if requested.x < margin
        || requested.x > crate::core::MAP_WIDTH - margin
        || requested.y < margin
        || requested.y > crate::core::MAP_HEIGHT - margin
    {
        return false;
    }
    let min_spacing_sq = ALTAR_MIN_SPACING * ALTAR_MIN_SPACING;
    !world
        .structures
        .iter()
        .any(|s| s.health.is_alive() && s.pos.distance_squared(requested) < min_spacing_sq)
}

/// Находит наилучшую клетку для постройки алтаря по алгоритму Maximin Distance от всех живых боссов.
/// Для любого количества боссов (соло или кооп) находит валидную клетку,
/// расстояние от которой до БЛИЖАЙШЕГО живого босса максимально.
pub fn find_furthest_altar_spawn(world: &World) -> Option<Vec2> {
    let mut living_boss_positions = [Vec2::ZERO; 8];
    let mut living_count = 0usize;

    if world.boss_health.is_alive() {
        living_boss_positions[living_count] = world.boss_pos;
        living_count += 1;
    }
    for player in &world.coop_bosses {
        if player.boss_health.is_alive() && living_count < 8 {
            living_boss_positions[living_count] = player.boss_pos;
            living_count += 1;
        }
    }

    if living_count == 0 {
        let center = snap_to_build_grid(Vec2::new(crate::core::MAP_WIDTH * 0.5, crate::core::MAP_HEIGHT * 0.5));
        return if can_place_blood_altar(world, center) { Some(center) } else { None };
    }

    let living_slice = &living_boss_positions[..living_count];
    let mut best_pos: Option<Vec2> = None;
    let mut max_min_dist_sq = -1.0f32;

    // Быстрый и точный поиск по 25 макро-слотам сетки арены (5x5).
    // Полностью синхронизирован с пространством действий и масками Директора.
    for slot in 0..crate::ai::neural_net::DIRECTOR_ALTAR_HEAD_SIZE.max(25).min(25) {
        let candidate = crate::ai::neural_net::DirectorBrainNet::altar_slot_position(slot);
        if !can_place_blood_altar(world, candidate) {
            continue;
        }

        let mut min_dist_sq = f32::MAX;
        for &boss_pos in living_slice {
            let d_sq = candidate.distance_squared(boss_pos);
            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
            }
        }

        if min_dist_sq > max_min_dist_sq {
            max_min_dist_sq = min_dist_sq;
            best_pos = Some(candidate);
        }
    }

    // Резервный поиск по 4 углам, если все 25 макро-слотов плотно заняты
    if best_pos.is_none() {
        let corners = [
            Vec2::new(70.0, 70.0),
            Vec2::new(crate::core::MAP_WIDTH - 70.0, 70.0),
            Vec2::new(70.0, crate::core::MAP_HEIGHT - 70.0),
            Vec2::new(crate::core::MAP_WIDTH - 70.0, crate::core::MAP_HEIGHT - 70.0),
        ];
        for corner in corners {
            let candidate = snap_to_build_grid(corner);
            if can_place_blood_altar(world, candidate) {
                let mut min_dist_sq = f32::MAX;
                for &boss_pos in living_slice {
                    let d_sq = candidate.distance_squared(boss_pos);
                    if d_sq < min_dist_sq {
                        min_dist_sq = d_sq;
                    }
                }
                if min_dist_sq > max_min_dist_sq {
                    max_min_dist_sq = min_dist_sq;
                    best_pos = Some(candidate);
                }
            }
        }
    }

    best_pos
}

/// Пытается построить алтарь в запрошенной точке.
pub fn try_spawn_blood_altar(world: &mut World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    if !can_place_blood_altar(world, requested) {
        return false;
    }

    spawn_blood_altar(world, requested);
    true
}

pub fn spawn_blood_altar(world: &mut World, pos: Vec2) {
    spawn_tier_altar(world, pos, 1);
}

pub fn try_spawn_tier_altar(world: &mut World, requested: Vec2, tier: u8) -> bool {
    let requested = snap_to_build_grid(requested);
    if !can_place_blood_altar(world, requested) {
        return false;
    }
    spawn_tier_altar(world, requested, tier);
    true
}

pub fn spawn_tier_altar(world: &mut World, pos: Vec2, tier: u8) {
    let pos = snap_to_build_grid(pos);
    let tier = tier.clamp(1, MAX_ALTAR_TIER);
    let id = world.alloc_id();
    let carapace_level = world.directors.first().map_or(0, |director| director.grade_carapace);
    let mut resistances = Resistances {
        flat_armor: 12.0,
        physical_res: 0.40,
        fire_res: 0.20,
        ice_res: 0.20,
        poison_res: 1.0, // Невосприимчив к яду
        holy_res: -0.60, // Сильно уязвим к святому урону!
    };
    apply_carapace(&mut resistances, carapace_level);
    world.structures.push(StructureEntity {
        id,
        pos,
        health: Health::new(650.0 + (tier.saturating_sub(1) as f32 * 180.0)),
        resistances,
        faction: Faction::Director,
        radius: 30.0,
        structure_type: structure_type_for_altar_tier(tier),
        pulse_timer: 0.0,
        spawn_timer: 0.0,
        lifetime: -1.0,
        max_lifetime: -1.0,
        is_active: false,
        state_timer: 0.0,
    });

    world.spawn_particles(pos, 25, Color::new(0.6, 0.0, 0.8, 1.0), 90.0);
    world.add_floating_text(pos, format!("АЛТАРЬ ТИРА {}", tier), PURPLE, 1.2);
}

/// Проверяет, можно ли поставить Стену-Баррикаду (50x50 px)
pub fn can_place_wall(world: &World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    // Нельзя ставить прямо поверх Босса
    if distance_sq_to_wall_aabb(world.boss_pos, requested) < 18.0 * 18.0 {
        return false;
    }
    // Adjacent wall cells share an edge exactly; walls cannot overlap other structures.
    !world.structures.iter().any(|structure| {
        if !structure.health.is_alive() {
            return false;
        }
        if structure.structure_type == STRUCTURE_WALL {
            let dx = (structure.pos.x - requested.x).abs();
            let dy = (structure.pos.y - requested.y).abs();
            dx < BUILD_GRID_SIZE && dy < BUILD_GRID_SIZE
        } else if structure.structure_type == STRUCTURE_OBELISK {
            // Match the visible 300x300 obelisk boundary. Wall cells may touch its
            // edge, but may not overlap the sprite or leave a misleading inner ring.
            let half_extent = OBELISK_FOOTPRINT_HALF_EXTENT + WALL_RADIUS;
            (structure.pos.x - requested.x).abs() < half_extent
                && (structure.pos.y - requested.y).abs() < half_extent
        } else {
            distance_sq_to_wall_aabb(structure.pos, requested) < structure.radius * structure.radius
        }
    })
}

pub fn try_spawn_wall(world: &mut World, requested: Vec2) -> bool {
    let requested = snap_to_build_grid(requested);
    if !can_place_wall(world, requested) {
        return false;
    }
    spawn_wall(world, requested);
    true
}

pub fn spawn_wall(world: &mut World, pos: Vec2) {
    let pos = snap_to_build_grid(pos);
    let id = world.alloc_id();
    let carapace_level = world.directors.first().map_or(0, |director| director.grade_carapace);
    let mut resistances = Resistances {
        flat_armor: 14.0,
        physical_res: 0.35,
        fire_res: 0.20,
        ice_res: 0.50,
        poison_res: 1.0,
        holy_res: -0.20,
    };
    apply_carapace(&mut resistances, carapace_level);
    world.structures.push(StructureEntity {
        id,
        pos,
        health: Health::new(350.0),
        resistances,
        faction: Faction::Director,
        radius: WALL_RADIUS,
        structure_type: STRUCTURE_WALL,
        pulse_timer: 0.0,
        spawn_timer: 0.0,
        lifetime: WALL_LIFETIME,
        max_lifetime: WALL_LIFETIME,
        is_active: false,
        state_timer: 0.0,
    });

    world.spawn_particles(pos, 15, DARKGRAY, 60.0);
    world.add_floating_text(pos, "БАРРИКАДА".to_string(), LIGHTGRAY, 1.1);
}

/// Спавнит 4 Древних Угловых Обелиска (300x300 px) по углам карты
pub fn spawn_corner_obelisks(world: &mut World) {
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;
    let offset = 260.0;

    let corners = [
        Vec2::new(offset, offset),                 // Северо-Запад
        Vec2::new(map_w - offset, offset),         // Северо-Восток
        Vec2::new(offset, map_h - offset),         // Юго-Запад
        Vec2::new(map_w - offset, map_h - offset), // Юго-Восток
    ];

    for pos in corners.map(snap_to_build_grid) {
        let id = world.alloc_id();
        let carapace_level = world.directors.first().map_or(0, |director| director.grade_carapace);
        let mut resistances = Resistances {
            flat_armor: 15.0,
            physical_res: 0.35,
            fire_res: 0.20,
            ice_res: 0.20,
            poison_res: 1.0,
            holy_res: 0.0,
        };
        apply_carapace(&mut resistances, carapace_level);
        world.structures.push(StructureEntity {
            id,
            pos,
            health: Health::new(OBELISK_BASE_HP_PER_BOSS * world.living_boss_count().max(1) as f32),
            resistances,
            faction: Faction::Director,
            radius: OBELISK_RADIUS,
            structure_type: STRUCTURE_OBELISK,
            pulse_timer: 0.0,
            spawn_timer: 0.0,
            lifetime: -1.0,
            max_lifetime: -1.0,
            is_active: false,
            state_timer: 0.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obelisk_hp_scaling_and_relic_reward() {
        let mut world = World::new();
        // Босс 1 жив
        assert_eq!(world.living_boss_count(), 1);

        // Проверяем изначальное здоровье обелисков для 1 живого босса
        spawn_corner_obelisks(&mut world);
        assert_eq!(world.structures.len(), 4);
        for st in &world.structures {
            assert_eq!(st.health.max, 2400.0);
            assert_eq!(st.health.current, 2400.0);
            assert!(!st.is_active);
        }

        // Добавляем второго живого босса
        world.add_coop_boss(2, Vec2::new(1200.0, 1200.0), "Boss2".to_string(), 0);
        assert_eq!(world.living_boss_count(), 2);

        // При 2 боссах обелиски должны отмасштабироваться до 4800 HP
        for st in &world.structures {
            assert_eq!(st.health.max, 4800.0);
            assert_eq!(st.health.current, 4800.0);
        }

        // Активируем первый обелиск (на 3 минуты = 180с)
        let obelisk_id = world.structures[0].id;
        world.structures[0].is_active = true;
        world.structures[0].state_timer = OBELISK_CHARGE_SECONDS;
        assert_eq!(world.structures[0].state_timer, 180.0);

        // Записываем урон от активного босса (ID 1)
        world.record_obelisk_contribution(obelisk_id);
        assert_eq!(world.boss_inventory.passives.len(), 0);

        // Уничтожаем активный обелиск
        world.structures[0].health.current = 0.0;
        crate::ecs::systems::combat::update_combat(&mut world, 0.016);

        // Проверяем, что босс получил в награду Реликвию!
        assert_eq!(world.boss_inventory.passives.len(), 1);
        assert_eq!(world.boss_obelisks_deactivated_total, 1);

        // Проверяем, что обелиск деактивировался, а HP сбросилось до полного значения для 2 боссов
        let obelisk = world.structures.iter().find(|s| s.id == obelisk_id).unwrap();
        assert!(!obelisk.is_active);
        assert_eq!(obelisk.state_timer, 0.0);
        assert_eq!(obelisk.health.current, 4800.0);
    }
}
