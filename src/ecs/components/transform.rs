use macroquad::prelude::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct Position(pub Vec2);

#[derive(Debug, Clone, Copy)]
pub struct Velocity(pub Vec2);

#[derive(Debug, Clone, Copy)]
pub struct HitboxRadius(pub f32);

#[derive(Debug, Clone, Copy)]
pub struct MoveSpeed(pub f32);
