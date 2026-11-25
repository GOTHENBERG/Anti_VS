use macroquad::prelude::Vec2;

/// Вычисление силы отталкивания по закону обратных квадратов (Потенциальные поля)
pub fn calculate_repel_force(target: Vec2, obstacle: Vec2, max_radius: f32, strength: f32) -> Vec2 {
    let diff = target - obstacle;
    let dist_sq = diff.length_squared();
    let max_radius_sq = max_radius * max_radius;

    if dist_sq > max_radius_sq || dist_sq < 0.001 {
        return Vec2::ZERO;
    }

    let dist = dist_sq.sqrt();
    let dir = diff / dist;

    // Закон обратных квадратов: сила резко растет по мере приближения
    let normalized_dist = (dist / max_radius).clamp(0.05, 1.0);
    let factor = (1.0 - normalized_dist) / (normalized_dist * normalized_dist);

    dir * (strength * factor).min(strength * 4.0)
}

/// Вычисление силы притяжения (к кристаллам душ)
pub fn calculate_attract_force(source: Vec2, target: Vec2, max_radius: f32, strength: f32) -> Vec2 {
    let diff = target - source;
    let dist_sq = diff.length_squared();
    let max_radius_sq = max_radius * max_radius;

    if dist_sq > max_radius_sq || dist_sq < 0.001 {
        return Vec2::ZERO;
    }

    let dist = dist_sq.sqrt();
    let dir = diff / dist;

    // Плавное притяжение
    let factor = 1.0 - (dist / max_radius) * 0.5;
    dir * (strength * factor)
}

/// Вращение 2D вектора на угол angle (в радианах)
#[inline(always)]
pub fn rotate_vector(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Вычисляет точку упреждения перехвата (Interception Point) движущейся цели снарядом.
/// Решает точное кинематическое уравнение:
/// |(P0 + V * t) - S0|^2 = (v_proj * t)^2
/// Коэффициенты: a * t^2 + b * t + c = 0
/// a = |V|^2 - v_proj^2
/// b = 2 * (D . V), где D = P0 - S0
/// c = |D|^2
pub fn calculate_lead_target(
    shooter_pos: Vec2,
    target_pos: Vec2,
    target_vel: Vec2,
    proj_speed: f32,
) -> Option<Vec2> {
    if !proj_speed.is_finite() || proj_speed <= 0.0 {
        return None;
    }

    let d = target_pos - shooter_pos;
    let dist_sq = d.length_squared();

    // Если цель уже в упор (дистанция < 1 пикселя), точка встречи — сама цель
    if dist_sq < 1.0 {
        return Some(target_pos);
    }

    // Решаем квадратное уравнение в f64 и по устойчивой формуле (q / a, c / q),
    // исключая потерю точности при |4ac| << b^2 или близких скоростях (a -> 0).
    let dx = d.x as f64;
    let dy = d.y as f64;
    let vx = target_vel.x as f64;
    let vy = target_vel.y as f64;
    let vp = proj_speed as f64;

    let a = (vx * vx + vy * vy) - vp * vp;
    let b = 2.0 * (dx * vx + dy * vy);
    let c = dx * dx + dy * dy;

    // Особый случай: скорости равны (a ≈ 0) -> линейное уравнение b * t + c = 0
    if a.abs() < 1e-9 {
        if b.abs() < 1e-9 {
            return Some(target_pos);
        }
        let t = -c / b;
        return if t > 1e-7 {
            Some(target_pos + target_vel * (t as f32))
        } else {
            None
        };
    }

    let disc = b * b - 4.0 * a * c;
    if disc < -1e-9 {
        return None; // Снаряд физически не догонит цель
    }

    let disc_sqrt = disc.max(0.0).sqrt();
    let q = -0.5 * (b + if b >= 0.0 { disc_sqrt } else { -disc_sqrt });
    let t1 = q / a;
    let t2 = if q.abs() > 1e-12 { c / q } else { t1 };

    // Выбираем наименьшее положительное время встречи (t > 1e-7)
    let t = match (t1 > 1e-7, t2 > 1e-7) {
        (true, true) => t1.min(t2),
        (true, false) => t1,
        (false, true) => t2,
        (false, false) => return None,
    };

    Some(target_pos + target_vel * (t as f32))
}

/// Вычисляет нормализованное направление выстрела с упреждением.
/// При dist_sq < 1.0 возвращает безопасный единичный вектор направления на цель, исключая NaNs.
pub fn calculate_lead_direction(
    shooter_pos: Vec2,
    target_pos: Vec2,
    target_vel: Vec2,
    proj_speed: f32,
) -> Option<Vec2> {
    let d = target_pos - shooter_pos;
    if d.length_squared() < 1.0 {
        return Some(d.normalize_or_zero());
    }

    calculate_lead_target(shooter_pos, target_pos, target_vel, proj_speed)
        .map(|intercept| (intercept - shooter_pos).normalize_or_zero())
}

/// Вычисление поворота вектора скорости по методу Пропорциональной Навигации (Proportional Navigation).
/// Устраняет «собачью кривую» (Pure Pursuit), удерживая постоянный угол линии визирования (LOS).
///
/// Математика:
/// R = target_pos - proj_pos
/// V_rel = target_vel - proj_vel
/// Omega_LOS = (R_x * V_rel_y - R_y * V_rel_x) / |R|^2  (угловая скорость вращения линии визирования)
/// V_closing = - (R . V_rel) / |R|                      (скорость сближения)
/// a_turn = N * V_closing * Omega_LOS, N = 3..5         (нормальное ускорение True PN)
/// omega = a_turn / V_proj = N * (V_closing / V_proj) * Omega_LOS
pub fn calculate_proportional_navigation_vel(
    proj_pos: Vec2,
    proj_vel: Vec2,
    target_pos: Vec2,
    target_vel: Vec2,
    homing_strength: f32,
    dt: f32,
) -> Vec2 {
    let current_speed = proj_vel.length();
    if current_speed <= 1.0 {
        return proj_vel;
    }

    let to_target = target_pos - proj_pos;
    let dist_sq = to_target.length_squared();
    if dist_sq <= 4.0 {
        return proj_vel;
    }

    let current_dir = proj_vel / current_speed;
    let dist = dist_sq.sqrt();
    let los_dir = to_target / dist;

    let v_rel = target_vel - proj_vel;

    // Векторное произведение 2D: R x V_rel (аналитическая производная угла LOS)
    let cross_los = to_target.x * v_rel.y - to_target.y * v_rel.x;
    let omega_los = cross_los / dist_sq;

    // Скорость сближения: - (R . V_rel) / |R|
    let closing_speed = -(to_target.x * v_rel.x + to_target.y * v_rel.y) / dist;

    // Навигационная константа N = 4.0 (оптимум для ракет/снарядов)
    const N_NAV: f32 = 4.0;

    let commanded_turn_rate = if closing_speed > 0.0 && to_target.dot(current_dir) > 0.0 {
        // True Proportional Navigation (TPN)
        N_NAV * (closing_speed / current_speed) * omega_los
    } else {
        // Если цель сзади или удаляемся — плавный пропорциональный доворот в сторону линии визирования
        let align_cross = current_dir.x * los_dir.y - current_dir.y * los_dir.x;
        (align_cross * 5.0).clamp(-homing_strength, homing_strength)
    };

    // Ограничение максимальной угловой скорости маневренностью снаряда
    let max_turn = homing_strength * dt;
    let actual_turn = (commanded_turn_rate * dt).clamp(-max_turn, max_turn);

    rotate_vector(current_dir, actual_turn).normalize_or_zero() * current_speed
}

/// Взвешенная случайная выборка без возвращения (Weighted Random Sampling without replacement)
/// методом Exponential Race (Эфраимидис — Спиракис в численно устойчивой log-форме):
/// key_i = -ln(U_i) / w_i, выбираются k элементов с наименьшими ключами.
/// Полностью исключает `powf`, underflow в f32 и сдвиги массива при удалении.
pub fn sample_weighted_without_replacement<T: Clone>(items: &[(T, f32)], k: usize) -> Vec<T> {
    if k == 0 || items.is_empty() {
        return Vec::new();
    }

    let mut top: Vec<(f32, usize)> = Vec::with_capacity(k);
    for (idx, (_, weight)) in items.iter().enumerate() {
        if *weight <= 0.0 || !weight.is_finite() {
            continue;
        }
        let u = crate::core::rand::gen_range(1e-7f32, 1.0f32);
        let key = -u.ln() / *weight;

        if top.len() < k {
            top.push((key, idx));
            if top.len() == k {
                top.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
            }
        } else if key < top[k - 1].0 {
            top[k - 1] = (key, idx);
            let mut j = k - 1;
            while j > 0 && top[j].0 < top[j - 1].0 {
                top.swap(j, j - 1);
                j -= 1;
            }
        }
    }

    if top.len() < k {
        top.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    }

    top.into_iter().map(|(_, idx)| items[idx].0.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_lead_perpendicular_matches_analytical() {
        let shooter = Vec2::ZERO;
        let target = Vec2::new(600.0, 0.0);
        let target_vel = Vec2::new(0.0, 300.0);
        let proj_speed = 600.0;

        // b = 0, a = 300^2 - 600^2 = -270000, c = 600^2 = 360000
        // t^2 = 360000 / 270000 = 4/3 -> t = sqrt(4/3) ≈ 1.1547005
        // intercept.y = 300 * sqrt(4/3) ≈ 346.41016
        let expected_t = (4.0f32 / 3.0f32).sqrt();
        let expected_lead_y = 300.0 * expected_t;

        let intercept = calculate_lead_target(shooter, target, target_vel, proj_speed).expect("Interception must exist");
        assert!((intercept.x - 600.0).abs() < 1e-4);
        assert!((intercept.y - expected_lead_y).abs() < 1e-2);
        assert!((intercept.y - 346.41).abs() < 0.1);

        let dir = calculate_lead_direction(shooter, target, target_vel, proj_speed).expect("Direction must exist");
        let expected_dir = (Vec2::new(600.0, expected_lead_y)).normalize();
        assert!((dir.x - expected_dir.x).abs() < 1e-4);
        assert!((dir.y - expected_dir.y).abs() < 1e-4);
    }

    #[test]
    fn test_calculate_lead_retreating_and_approaching() {
        let shooter = Vec2::ZERO;
        let target = Vec2::new(600.0, 0.0);
        let proj_speed = 600.0;

        // Удаление: V = (300, 0) -> t = 600 / (600 - 300) = 2.0 s -> intercept = (1200, 0), lead = 600
        let target_vel_away = Vec2::new(300.0, 0.0);
        let intercept_away = calculate_lead_target(shooter, target, target_vel_away, proj_speed).unwrap();
        assert!((intercept_away.x - 1200.0).abs() < 1e-3);
        assert!(intercept_away.y.abs() < 1e-3);

        // Приближение: V = (-300, 0) -> t = 600 / (600 + 300) = 2/3 s -> intercept = (400, 0), lead = 200
        let target_vel_toward = Vec2::new(-300.0, 0.0);
        let intercept_toward = calculate_lead_target(shooter, target, target_vel_toward, proj_speed).unwrap();
        assert!((intercept_toward.x - 400.0).abs() < 1e-3);
        assert!(intercept_toward.y.abs() < 1e-3);
    }

    #[test]
    fn test_calculate_lead_point_blank_safe() {
        let shooter = Vec2::new(100.0, 100.0);
        let target = Vec2::new(100.5, 100.0); // dist = 0.5 < 1.0
        let target_vel = Vec2::new(200.0, 0.0);
        let proj_speed = 500.0;

        let dir = calculate_lead_direction(shooter, target, target_vel, proj_speed).unwrap();
        assert!((dir.x - 1.0).abs() < 1e-5);
        assert!(dir.y.abs() < 1e-5);
        assert!(!dir.x.is_nan() && !dir.y.is_nan());
    }

    #[test]
    fn test_calculate_lead_impossible_outrun() {
        let shooter = Vec2::ZERO;
        let target = Vec2::new(600.0, 0.0);
        let target_vel = Vec2::new(700.0, 0.0); // faster than projectile
        let proj_speed = 600.0;

        let res = calculate_lead_target(shooter, target, target_vel, proj_speed);
        assert!(res.is_none());
    }

    #[test]
    fn test_proportional_navigation_straight_when_on_course() {
        let proj_pos = Vec2::new(0.0, 0.0);
        let proj_vel = Vec2::new(400.0, 0.0);
        let target_pos = Vec2::new(200.0, 0.0);
        let target_vel = Vec2::ZERO;

        let next_vel = calculate_proportional_navigation_vel(proj_pos, proj_vel, target_pos, target_vel, 5.0, 0.016);
        assert!((next_vel.x - 400.0).abs() < 1e-3);
        assert!(next_vel.y.abs() < 1e-3);
    }

    #[test]
    fn test_proportional_navigation_turns_toward_intercept() {
        let proj_pos = Vec2::new(0.0, 0.0);
        let proj_vel = Vec2::new(400.0, 0.0);
        let target_pos = Vec2::new(200.0, 50.0);
        let target_vel = Vec2::new(0.0, 100.0); // drifting upwards

        let next_vel = calculate_proportional_navigation_vel(proj_pos, proj_vel, target_pos, target_vel, 8.0, 0.016);
        // Должен начать поворачивать вверх (+y)
        assert!(next_vel.y > 0.0);
        assert!((next_vel.length() - 400.0).abs() < 1e-2);
    }

    #[test]
    fn test_exponential_race_weighted_sampling_without_replacement() {
        let items = vec![
            (0usize, 500.0f32),
            (1usize, 50.0f32),
            (2usize, 10.0f32),
            (3usize, 0.0001f32),
            (4usize, 0.0f32), // нулевой вес не должен выбираться
        ];

        let mut counts = [0usize; 5];
        for _ in 0..1000 {
            let pick = sample_weighted_without_replacement(&items, 2);
            assert_eq!(pick.len(), 2);
            assert_ne!(pick[0], pick[1], "Выборка обязана быть без возвращения");
            for id in pick {
                counts[id] += 1;
            }
        }

        assert_eq!(counts[4], 0, "Элемент с нулевым весом не может быть выбран");
        assert!(counts[0] > counts[1] && counts[1] > counts[2] && counts[2] > counts[3],
            "Порядок частот обязан строго следовать весам: {:?}", counts);
    }

    #[test]
    fn test_calculate_lead_boundary_and_citardauq_stability() {
        // 1. Граничные некорректные скорости снаряда
        assert!(calculate_lead_target(Vec2::ZERO, Vec2::new(100.0, 0.0), Vec2::ZERO, 0.0).is_none());
        assert!(calculate_lead_target(Vec2::ZERO, Vec2::new(100.0, 0.0), Vec2::ZERO, -50.0).is_none());
        assert!(calculate_lead_target(Vec2::ZERO, Vec2::new(100.0, 0.0), Vec2::ZERO, f32::NAN).is_none());

        // 2. Равные скорости (a == 0): перпендикулярно/под углом навстречу
        let intercept_eq = calculate_lead_target(
            Vec2::ZERO,
            Vec2::new(600.0, 0.0),
            Vec2::new(-300.0, 400.0), // |V| = 500 == v_p
            500.0,
        )
        .expect("Equal speed closing shot must have solution");
        let t_eq = (intercept_eq - Vec2::ZERO).length() / 500.0;
        let target_at_t = Vec2::new(600.0, 0.0) + Vec2::new(-300.0, 400.0) * t_eq;
        assert!(
            intercept_eq.distance(target_at_t) < 1e-3,
            "Equal-speed intercept residual too large: {} vs {}",
            intercept_eq,
            target_at_t
        );

        // 3. Режим сильной взаимной компенсации (|4ac| << b^2 и малый a)
        let shooter = Vec2::ZERO;
        let target = Vec2::new(2.0, 0.0);
        let vel = Vec2::new(-9999.0, 0.0);
        let v_p = 10000.0;
        let intercept = calculate_lead_target(shooter, target, vel, v_p)
            .expect("High-b Citardauq root must stay finite and valid");
        let expected_t = 2.0 / (10000.0 + 9999.0);
        let expected_x = 2.0 - 9999.0 * expected_t;
        assert!(
            (intercept.x - expected_x).abs() < 1e-4,
            "Citardauq precision mismatch: got {}, expected {}",
            intercept.x,
            expected_x
        );
        assert!(intercept.is_finite());

        // 4. Почти равные скорости при расхождении (a -> 0+ и невозможность перехвата)
        assert!(calculate_lead_target(
            Vec2::ZERO,
            Vec2::new(600.0, 0.0),
            Vec2::new(500.0, 0.0),
            500.0
        )
        .is_none());
    }
}
