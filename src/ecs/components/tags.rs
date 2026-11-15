#[derive(Clone, Copy, Debug, Default)]
pub struct IsBoss;

#[derive(Clone, Copy, Debug, Default)]
pub struct IsMob;

#[derive(Clone, Copy, Debug, Default)]
pub struct IsSoulGem {
    pub xp_value: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IsProjectile {
    pub damage: f32,
    pub lifetime: f32,
    pub pierce_remaining: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Destructible;

#[derive(Clone, Copy, Debug, Default)]
pub struct Structure;
