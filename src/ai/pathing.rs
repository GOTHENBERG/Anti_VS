use macroquad::prelude::*;
use crate::core::{MAP_WIDTH, MAP_HEIGHT};
use crate::ecs::world::World;

/// Данные навигации по ключевым точкам (Waypoints) и Алтарям Крови
#[derive(Clone, Debug)]
pub struct AltarNavData {
    pub altar_count: usize,
    pub closest_altar_pos: Option<Vec2>,
    pub second_altar_pos: Option<Vec2>,
    /// Время (в секундах) перехода от текущей позиции Босса до Точки A (ближайший алтарь)
    pub time_to_closest_altar: f32,
    /// Время (в секундах) перехода от Точки Y (Алтарь 1) до Точки Z (Алтарь 2 или кластер опыта)
    pub time_chained_y_to_z: f32,
}

/// Модуль навигации, оценки траекторий и кинематического времени перемещения (Kinematic Waypoint Estimation)
pub struct WaypointPlanner;

impl WaypointPlanner {
    /// Рассчитывает прогнозируемое время перемещения от точки From до точки To
    /// с учетом эффективной скорости Босса, сопротивления плотности толпы и кулдауна рывка.
    pub fn estimate_travel_time(world: &World, from: Vec2, to: Vec2) -> f32 {
        let diff = to - from;
        let dist = diff.length();
        if dist < 1.0 {
            return 0.0;
        }

        // Базовая скорость босса с учетом пассивок и возвышений
        let base_speed = world.boss_inventory.get_movement_speed(160.0);

        // Проверяем плотность мобов вдоль отрезка (трассировка пути)
        // Сэмплируем 3 контрольные точки вдоль прямой для оценки сопротивления среды
        let mut crowd_drag = 1.0f32;
        let samples = 3;
        for s in 1..=samples {
            let t = s as f32 / (samples as f32 + 1.0);
            let sample_pos = from.lerp(to, t);
            let nearby = world.spatial_grid.query_nearby(sample_pos, 70.0).len();
            if nearby > 5 {
                // Плотная толпа на пути замедляет продвижение на 15-40%
                crowd_drag += (nearby as f32 * 0.03).min(0.6);
            }
        }

        let effective_speed = (base_speed / crowd_drag).max(40.0);

        // Учитываем наличие готового рывка: если рывок готов, он срезает 180 пикселей пути мгновенно!
        let distance_after_dash = if world.boss_dash_cooldown <= 0.0 && dist > 180.0 {
            dist - 180.0
        } else {
            dist
        };

        distance_after_dash / effective_speed
    }

    /// Рассчитывает время перемещения по цепочке точек:
    /// Сначала: Текущая позиция Босса -> Точка A
    /// Затем: Точка Y -> Точка Z (например, от Алтаря 1 до Алтаря 2 или скопления опыта)
    pub fn estimate_chained_travel_time(world: &World, to_a: Vec2, from_y: Vec2, to_z: Vec2) -> (f32, f32) {
        let time_to_a = Self::estimate_travel_time(world, world.boss_pos, to_a);
        let time_y_to_z = Self::estimate_travel_time(world, from_y, to_z);
        (time_to_a, time_y_to_z)
    }

    /// Извлекает точные координаты всех активных Алтарей Крови и рассчитывает
    /// навигационные маршруты от точки Босса до Алтаря A, и от Алтаря 1 (Y) до Алтаря 2 (Z)
    pub fn calculate_altar_navigation(world: &World) -> AltarNavData {
        let boss_pos = world.boss_pos;

        // Находим все живые алтари
        let mut altars: Vec<Vec2> = world.structures
            .iter()
            .filter(|s| s.structure_type == 1 && s.health.is_alive())
            .map(|s| s.pos)
            .collect();

        // Сортируем по удаленности от Босса
        altars.sort_by(|a, b| {
            let d_a = a.distance_squared(boss_pos);
            let d_b = b.distance_squared(boss_pos);
            d_a.partial_cmp(&d_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let altar_count = altars.len();
        let closest = altars.first().copied();
        let second = altars.get(1).copied();

        let (time_to_closest, time_chained) = match (closest, second) {
            (Some(a), Some(b)) => {
                // Путь: Босс -> Алтарь A, затем Алтарь A (Y) -> Алтарь B (Z)
                let t_a = Self::estimate_travel_time(world, boss_pos, a);
                let t_chain = Self::estimate_travel_time(world, a, b);
                (t_a, t_chain)
            }
            (Some(a), None) => {
                let t_a = Self::estimate_travel_time(world, boss_pos, a);
                // Если второго алтаря нет, в качестве точки Z берем центр ближайшего скопления кристаллов
                let z_target = Self::find_densest_gem_cluster(world, a).unwrap_or(Vec2::new(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5));
                let t_chain = Self::estimate_travel_time(world, a, z_target);
                (t_a, t_chain)
            }
            (None, _) => (0.0, 0.0),
        };

        AltarNavData {
            altar_count,
            closest_altar_pos: closest,
            second_altar_pos: second,
            time_to_closest_altar: time_to_closest,
            time_chained_y_to_z: time_chained,
        }
    }

    /// Находит ближайший к точке центр скопления кристаллов опыта
    fn find_densest_gem_cluster(world: &World, near: Vec2) -> Option<Vec2> {
        if world.gems.is_empty() {
            return None;
        }
        let mut closest_gem = world.gems[0].pos;
        let mut min_d_sq = closest_gem.distance_squared(near);
        for g in &world.gems {
            let d_sq = g.pos.distance_squared(near);
            if d_sq < min_d_sq {
                min_d_sq = d_sq;
                closest_gem = g.pos;
            }
        }
        Some(closest_gem)
    }
}
