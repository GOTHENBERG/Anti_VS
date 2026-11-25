use macroquad::prelude::*;
use crate::ecs::components::combat::{Faction, Health, Resistances};
use crate::ecs::world::{StructureEntity, World};

pub const STRUCTURE_ALTAR: u8 = 1;
pub const STRUCTURE_WALL: u8 = 2;
pub const STRUCTURE_OBELISK: u8 = 3;
pub const STRUCTURE_TIER_ALTAR_BASE: u8 = 20;
pub const MAX_ALTAR_TIER: u8 = 16;

pub const BLOOD_ALTAR_COST: f32 = 50.0;
pub const ALTAR_GRID_SIZE: f32 = 100.0;
pub const ALTAR_HALF_EXTENT: f32 = 50.0; // Хитбокс алтаря 100x100 px
pub const ALTAR_RADIUS: f32 = 50.0;      // Физический радиус для коллизий и урона
pub const ALTAR_MIN_SPACING: f32 = ALTAR_GRID_SIZE;
pub const BUILD_GRID_SIZE: f32 = 50.0;
pub const ALTAR_BASE_COOLDOWN_SECONDS: f32 = 14.0;
pub const ALTAR_COOLDOWN_TIER_STEP: f32 = 7.0;

pub const WALL_COST: f32 = 10.0;
pub const WALL_MAX_COOLDOWN: f32 = 2.0;
pub const WALL_LIFETIME: f32 = 60.0;
pub const WALL_RADIUS: f32 = 25.0; // 50x50 px баррикада
pub const WALL_BASE_HP: f32 = 1050.0; // Базовое HP баррикады (было 350.0, увеличено на 200%)
pub const OBELISK_RADIUS: f32 = 75.0; // 300x300 px визуальный размер
pub const OBELISK_FOOTPRINT_HALF_EXTENT: f32 = OBELISK_RADIUS * 2.0;
pub const OBELISK_BASE_HP_PER_BOSS: f32 = 4800.0; // Базовое HP обелиска на босса (было 2400.0, увеличено на 100%)
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
    let squad_cost = crate::content::mobs::get_squad_config(tier).cost;
    (squad_cost * 2.5).max(BLOOD_ALTAR_COST)
}

pub fn altar_cooldown(tier: u8) -> f32 {
    let tier = tier.clamp(1, MAX_ALTAR_TIER);
    ALTAR_BASE_COOLDOWN_SECONDS + tier.saturating_sub(1) as f32 * ALTAR_COOLDOWN_TIER_STEP
}

pub fn altar_spawn_interval(tier: u8) -> f32 {
    let squad_count = crate::content::mobs::get_squad_config(tier).count.max(1);
    8.0 / squad_count as f32
}

/// Привязывает координату центра алтаря 100x100 к узлам сетки 100 px (2x2 клетки 50 px)
pub fn snap_altar_to_grid(pos: Vec2) -> Vec2 {
    let snap_axis = |value: f32, extent: f32| {
        ((value / ALTAR_GRID_SIZE).floor() * ALTAR_GRID_SIZE + ALTAR_HALF_EXTENT)
            .clamp(ALTAR_HALF_EXTENT + 5.0, extent - ALTAR_HALF_EXTENT - 5.0)
    };
    Vec2::new(snap_axis(pos.x, crate::core::MAP_WIDTH), snap_axis(pos.y, crate::core::MAP_HEIGHT))
}

/// Converts a world coordinate to the center of its 50 px placement cell.
pub fn snap_to_build_grid(pos: Vec2) -> Vec2 {
    let snap_axis = |value: f32, extent: f32| {
        ((value / BUILD_GRID_SIZE).floor() * BUILD_GRID_SIZE + BUILD_GRID_SIZE * 0.5)
            .clamp(BUILD_GRID_SIZE * 0.5, extent - BUILD_GRID_SIZE * 0.5)
    };
    Vec2::new(snap_axis(pos.x, crate::core::MAP_WIDTH), snap_axis(pos.y, crate::core::MAP_HEIGHT))
}

pub fn apply_carapace(resistances: &mut Resistances, level: u32) {
    resistances.flat_armor += level as f32 * 2.5;
    resistances.physical_res = (resistances.physical_res + level as f32 * 0.05).min(0.70);
}

/// Applies a newly purchased Carapace level to already placed Director structures, heroes, and mobs.
pub fn upgrade_structure_carapace(world: &mut World, levels: u32) {
    if levels == 0 {
        return;
    }
    // 1. Все строения Директора (баррикады, алтари, обелиски, клетки)
    for structure in &mut world.structures {
        if structure.faction == Faction::Director {
            apply_carapace(&mut structure.resistances, levels);
        }
    }
    // 2. Все живые мобы и герои Директора
    for mob in &mut world.mobs {
        apply_carapace(&mut mob.resistances, levels);
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

/// Проверяет, можно ли поставить алтарь (100x100 px) в запрошенной точке.
/// Алтари можно ставить вплотную к чему угодно (к стенам, другим алтарям и обелискам).
/// Запрещено накладываться на другие алтари, стены и обелиски.
pub fn can_place_blood_altar(world: &World, requested: Vec2) -> bool {
    let requested = snap_altar_to_grid(requested);
    let margin = ALTAR_HALF_EXTENT;
    if requested.x < margin
        || requested.x > crate::core::MAP_WIDTH - margin
        || requested.y < margin
        || requested.y > crate::core::MAP_HEIGHT - margin
    {
        return false;
    }
    !world.structures.iter().any(|s| {
        if !s.health.is_alive() {
            return false;
        }
        let dx = (s.pos.x - requested.x).abs();
        let dy = (s.pos.y - requested.y).abs();
        if is_altar_structure_type(s.structure_type) {
            // Другой алтарь 100x100: расстояние между центрами должно быть не менее 100 по X или Y
            dx < 95.0 && dy < 95.0
        } else if s.structure_type == STRUCTURE_WALL {
            // Стена 50x50: расстояние между центром алтаря (50) и стены (25) не менее 75 по X или Y
            dx < 70.0 && dy < 70.0
        } else if s.structure_type == STRUCTURE_OBELISK {
            // Обелиск 150x150 (half 75): расстояние не менее 75 + 50 = 125 по X или Y
            dx < 120.0 && dy < 120.0
        } else {
            dx < (s.radius + ALTAR_HALF_EXTENT - 5.0) && dy < (s.radius + ALTAR_HALF_EXTENT - 5.0)
        }
    })
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
        let center = snap_altar_to_grid(Vec2::new(crate::core::MAP_WIDTH * 0.5, crate::core::MAP_HEIGHT * 0.5));
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
            let candidate = snap_altar_to_grid(corner);
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
    let requested = snap_altar_to_grid(requested);
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
    let requested = snap_altar_to_grid(requested);
    if !can_place_blood_altar(world, requested) {
        return false;
    }
    spawn_tier_altar(world, requested, tier);
    true
}

pub fn spawn_tier_altar(world: &mut World, pos: Vec2, tier: u8) {
    let pos = snap_altar_to_grid(pos);
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
        radius: ALTAR_RADIUS,
        structure_type: structure_type_for_altar_tier(tier),
        pulse_timer: 0.0,
        spawn_timer: 0.0,
        lifetime: -1.0,
        max_lifetime: -1.0,
        is_active: false,
        state_timer: 0.0,
    });

    if !world.is_headless {
        world.spawn_particles(pos, 35, Color::new(0.6, 0.0, 0.8, 1.0), 120.0);
        world.add_floating_text(pos, format!("АЛТАРЬ ТИРА {}", tier), PURPLE, 1.2);
    }
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
        let dx = (structure.pos.x - requested.x).abs();
        let dy = (structure.pos.y - requested.y).abs();
        if structure.structure_type == STRUCTURE_WALL {
            dx < 45.0 && dy < 45.0
        } else if structure.structure_type == STRUCTURE_OBELISK {
            // Match the visible 300x300 obelisk boundary. Wall cells may touch its
            // edge, but may not overlap the sprite or leave a misleading inner ring.
            let half_extent = OBELISK_FOOTPRINT_HALF_EXTENT + WALL_RADIUS;
            dx < (half_extent - 5.0) && dy < (half_extent - 5.0)
        } else if is_altar_structure_type(structure.structure_type) {
            // Алтарь 100x100: стена 50x50 не может пересекать его хитбокс (half 50 + half 25 = 75)
            dx < 70.0 && dy < 70.0
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
        health: Health::new(WALL_BASE_HP),
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
            assert_eq!(st.health.max, 4800.0);
            assert_eq!(st.health.current, 4800.0);
            assert!(!st.is_active);
        }

        // Добавляем второго живого босса
        world.add_coop_boss(2, Vec2::new(1200.0, 1200.0), "Boss2".to_string(), 0);
        assert_eq!(world.living_boss_count(), 2);

        // При 2 боссах обелиски должны отмасштабироваться до 9600 HP (4800 * 2)
        for st in &world.structures {
            assert_eq!(st.health.max, 9600.0);
            assert_eq!(st.health.current, 9600.0);
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

        // Проверяем, что ОБА босса получили в награду Реликвию (независимо от того, кто наносил урон)!
        assert_eq!(world.boss_inventory.passives.len(), 1);
        assert_eq!(world.coop_bosses[0].boss_inventory.passives.len(), 1);
        assert_eq!(world.boss_obelisks_deactivated_total, 1);

        // Проверяем, что обелиск деактивировался, а HP сбросилось до полного значения для 2 боссов
        let obelisk = world.structures.iter().find(|s| s.id == obelisk_id).unwrap();
        assert!(!obelisk.is_active);
        assert_eq!(obelisk.state_timer, 0.0);
        assert_eq!(obelisk.health.current, 9600.0);
    }

    #[test]
    fn test_obelisk_cannot_activate_while_hero_alive() {
        let mut world = World::new();
        world.is_headless = true;
        spawn_corner_obelisks(&mut world);

        // All obelisks are inactive initially
        for st in &world.structures {
            assert!(!st.is_active);
        }

        // Add a living hero mob to the world
        let mob_id = world.alloc_id();
        let mut hero = crate::ecs::world::MobEntity::new_test(mob_id, Vec2::new(500.0, 500.0), 17);
        hero.is_hero = true;
        world.mobs.push(hero);

        world.obelisk_cycle_timer = 0.5;

        // Run update_world_structures for dt = 1.0
        crate::ecs::systems::skills::update_world_structures(&mut world, 1.0);

        // Because hero is alive, obelisk timer does not tick down and no obelisk activates!
        assert_eq!(world.obelisk_cycle_timer, 0.5);
        for st in &world.structures {
            assert!(!st.is_active, "Obelisk must NOT activate while a hero is alive");
        }

        // Kill the hero
        world.mobs[0].health.current = 0.0;

        // Now run update_world_structures
        crate::ecs::systems::skills::update_world_structures(&mut world, 1.0);

        // One obelisk should now become active!
        let active_count = world.structures.iter().filter(|s| s.is_active).count();
        assert_eq!(active_count, 1, "After hero dies, obelisk cycle can activate an obelisk");
    }

    #[test]
    fn test_obelisk_relic_draft_human_boss() {
        let mut world = World::new();
        world.boss_human_controlled = true;
        spawn_corner_obelisks(&mut world);

        let obelisk_id = world.structures[0].id;
        world.structures[0].is_active = true;
        world.record_obelisk_contribution(obelisk_id);

        assert_eq!(world.boss_inventory.passives.len(), 0);
        assert_eq!(world.boss_pending_relic_choices, 0);
        assert!(world.boss_level_up_options.is_none());

        // Уничтожаем активный обелиск
        world.structures[0].health.current = 0.0;
        crate::ecs::systems::combat::update_combat(&mut world, 0.016);

        // Для живого игрока-человека не выдается реликвия автоматически,
        // а открывается интерфейс с выбором из 3 реликвий!
        assert_eq!(world.boss_pending_relic_choices, 1);
        assert!(world.boss_level_up_options.is_some());
        let options = world.boss_level_up_options.as_ref().unwrap();
        assert_eq!(options.len(), 3);
        for opt in options {
            if let crate::ecs::components::inventory::PerkChoice::UpgradePassive(k) = opt {
                assert!(crate::content::passives::is_legendary_relic(*k));
            }
        }
        // Пассивка ещё не добавлена (игрок выбирает)
        assert_eq!(world.boss_inventory.passives.len(), 0);

        // Игрок выбирает карту #0
        crate::ecs::systems::coop::apply_player_perk_choice_by_index(&mut world, 0, 0);

        // Выбор применен!
        assert_eq!(world.boss_pending_relic_choices, 0);
        assert!(world.boss_level_up_options.is_none());
        assert_eq!(world.boss_inventory.passives.len(), 1);
        assert!(crate::content::passives::is_legendary_relic(world.boss_inventory.passives[0].kind));
    }

    #[test]
    fn test_map_dimensions_traversal_time_and_obelisk_activation_audio() {
        use crate::core::{MAP_WIDTH, MAP_HEIGHT, MAP_DIAGONAL};

        // 1. Проверяем расчет времени пробега по диагонали карты при базовой скорости босса 160 px/s
        let boss_speed = 160.0f32;
        let diagonal = (MAP_WIDTH * MAP_WIDTH + MAP_HEIGHT * MAP_HEIGHT).sqrt();
        assert!((diagonal - MAP_DIAGONAL).abs() < 1.0, "MAP_DIAGONAL must match sqrt(W^2 + H^2)");

        let travel_time_seconds = diagonal / boss_speed;
        let travel_time_minutes = travel_time_seconds / 60.0;

        // Требование пользователя: 3 - 3.5 минуты (180 - 210 секунд)
        assert!(
            travel_time_seconds >= 180.0 && travel_time_seconds <= 210.0,
            "Corner-to-corner travel time must be 3-3.5 minutes! Got {:.2} min ({:.1} s)",
            travel_time_minutes,
            travel_time_seconds
        );

        // 2. Проверяем активацию обелиска: звук активации и уведомления
        let mut world = World::new();
        world.is_headless = false;
        spawn_corner_obelisks(&mut world);

        world.obelisk_cycle_timer = 0.0;
        // Запускаем обновление структур
        crate::ecs::systems::skills::update_world_structures(&mut world, 0.1);

        // Обелиск должен стать активным
        let active_obelisk = world.structures.iter().find(|s| s.is_active);
        assert!(active_obelisk.is_some(), "An obelisk must activate when cycle timer expires");

        // Должен быть воспроизведен звук активации разлома
        assert!(
            world.audio_events.contains(&crate::audio::AudioEvent::ObeliskActivate),
            "AudioEvent::ObeliskActivate must be triggered on rift activation"
        );

        // Должно быть отправлено уведомление игроку-боссу
        assert!(
            world.floating_texts.iter().any(|ft| ft.text.contains("ТРЕЩИТ ЗЕМЛЯ") || ft.text.contains("ИНФЕРНАЛЬНЫЙ РАЗЛОМ")),
            "Notification floating text must be dispatched to player"
        );
    }

    #[test]
    fn test_barricade_hp_and_carapace_buff_propagation() {
        let mut world = World::new();
        assert_eq!(WALL_BASE_HP, 1050.0);
        assert_eq!(OBELISK_BASE_HP_PER_BOSS, 4800.0);

        // 1. Ставим баррикаду
        let wall_pos = snap_to_build_grid(Vec2::new(300.0, 300.0));
        spawn_wall(&mut world, wall_pos);
        assert_eq!(world.structures.len(), 1);
        let wall = &world.structures[0];
        assert_eq!(wall.health.max, 1050.0, "Баррикада должна иметь 1050 HP (+200% от 350.0)");
        assert_eq!(wall.resistances.flat_armor, 14.0);
        assert_eq!(wall.resistances.physical_res, 0.35);

        // 2. Ставим алтарь и обелиск
        let altar_pos = snap_altar_to_grid(Vec2::new(550.0, 550.0));
        spawn_blood_altar(&mut world, altar_pos);
        spawn_corner_obelisks(&mut world);

        // 3. Добавляем обычного моба и героя
        let mob_id = world.alloc_id();
        let mut mob = crate::ecs::world::MobEntity::new_test(mob_id, Vec2::new(700.0, 700.0), 1);
        mob.resistances.flat_armor = 0.0;
        mob.resistances.physical_res = 0.05;
        world.mobs.push(mob);

        let hero_id = world.alloc_id();
        let mut hero = crate::ecs::world::MobEntity::new_test(hero_id, Vec2::new(800.0, 800.0), 17);
        hero.is_hero = true;
        hero.resistances.flat_armor = 35.0;
        hero.resistances.physical_res = 0.50;
        world.mobs.push(hero);

        // 4. Директор прокачивает Панцирь (Carapace) на 2 уровня:
        // +2.5 * 2 = +5.0 к броне, +0.05 * 2 = +0.10 к физ. резисту
        upgrade_structure_carapace(&mut world, 2);

        // Проверяем баррикаду:
        let wall = &world.structures[0];
        assert_eq!(wall.resistances.flat_armor, 19.0, "Баррикада должна получить +5 брони");
        assert_eq!(wall.resistances.physical_res, 0.45, "Баррикада должна получить +10% физ. реза");

        // Проверяем алтарь:
        let altar = &world.structures[1];
        assert_eq!(altar.resistances.flat_armor, 17.0, "Алтарь должен получить +5 брони");
        assert_eq!(altar.resistances.physical_res, 0.50, "Алтарь должен получить +10% физ. реза");

        // Проверяем обелиск:
        let obelisk = &world.structures[2];
        assert_eq!(obelisk.resistances.flat_armor, 20.0, "Обелиск должен получить +5 брони");
        assert_eq!(obelisk.resistances.physical_res, 0.45, "Обелиск должен получить +10% физ. реза");

        // Проверяем обычного моба:
        let m = &world.mobs[0];
        assert_eq!(m.resistances.flat_armor, 5.0, "Моб должен получить +5 брони");
        assert_eq!(m.resistances.physical_res, 0.15, "Моб должен получить +10% физ. реза");

        // Проверяем героя:
        let h = &world.mobs[1];
        assert_eq!(h.resistances.flat_armor, 40.0, "Герой должен получить +5 брони");
        assert_eq!(h.resistances.physical_res, 0.60, "Герой должен получить +10% физ. реза");
    }

    #[test]
    fn test_barricade_sliding_physics() {
        let mut world = World::new();
        world.is_headless = true;
        spawn_wall(&mut world, Vec2::new(1000.0, 1000.0));
        let wall_pos = world.structures[0].pos;

        // Босс слева от баррикады на расстоянии касания (43px - min_dist)
        // и движется по диагонали (вправо-вниз) прямо на баррикаду
        world.boss_pos = Vec2::new(wall_pos.x - 40.0, wall_pos.y);
        world.boss_vel = Vec2::new(150.0, 150.0);

        crate::ecs::systems::movement::update_movement(&mut world, 0.016);

        // Босс не должен провалиться сквозь стену
        let dist = world.boss_pos.distance(wall_pos);
        assert!(dist >= 42.9, "Босс не должен проваливаться внутрь баррикады! Dist: {:.2}", dist);

        // Составляющая скорости, направленная внутрь стены (+X), должна быть погашена,
        // а тангенциальная составляющая (+Y) сохранена для плавного скольжения вдоль стены
        assert!(world.boss_vel.x <= 0.01, "Скорость в стену должна быть погашена: {:.2}", world.boss_vel.x);
        assert!(world.boss_vel.y > 100.0, "Скольжение вдоль стены должно сохраниться: {:.2}", world.boss_vel.y);
    }
}
