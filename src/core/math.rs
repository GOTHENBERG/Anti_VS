use macroquad::prelude::Vec2;

pub fn calculate_repel_force(target: Vec2, obstacle: Vec2, max_radius: f32, strength: f32) -> Vec2 {
    let diff = target - obstacle;
    let dist_sq = diff.length_squared();
    let max_radius_sq = max_radius * max_radius;

    if dist_sq > max_radius_sq || dist_sq < 0.0001 {
        return Vec2::ZERO;
    }

    let dist = dist_sq.sqrt();
    let dir = diff / dist;
    // Сила обратно пропорциональна расстоянию (потенциальное поле)
    let factor = 1.0 - (dist / max_radius);
    dir * (strength * factor)
}

pub fn calculate_attract_force(source: Vec2, target: Vec2, max_radius: f32, strength: f32) -> Vec2 {
    let diff = target - source;
    let dist_sq = diff.length_squared();
    let max_radius_sq = max_radius * max_radius;

    if dist_sq > max_radius_sq || dist_sq < 0.0001 {
        return Vec2::ZERO;
    }

    let dist = dist_sq.sqrt();
    let dir = diff / dist;
    dir * strength
}
