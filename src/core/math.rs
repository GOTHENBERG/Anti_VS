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
