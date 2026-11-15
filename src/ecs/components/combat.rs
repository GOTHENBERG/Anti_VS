use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    Physical,
    Fire,
    Ice,
    Poison,
    Holy,
    True,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faction {
    Boss,
    Director,
    Neutral,
}

#[derive(Clone, Debug)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    pub fn take_damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

#[derive(Clone, Debug)]
pub struct Resistances {
    pub flat_armor: f32,
    pub physical_res: f32, // 0.0 .. 1.0 (процентное снижение)
    pub fire_res: f32,
    pub ice_res: f32,
    pub poison_res: f32,
    pub holy_res: f32,
}

impl Default for Resistances {
    fn default() -> Self {
        Self {
            flat_armor: 0.0,
            physical_res: 0.0,
            fire_res: 0.0,
            ice_res: 0.0,
            poison_res: 0.0,
            holy_res: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DamagePacket {
    pub source_id: Option<usize>,
    pub target_id: usize,
    pub amount: f32,
    pub damage_type: DamageType,
    pub armor_penetration: f32, // e.g. 0.20 = 20%
}

impl Resistances {
    pub fn calculate_damage(&self, packet: &DamagePacket) -> f32 {
        if packet.damage_type == DamageType::True {
            return packet.amount;
        }

        let base_res = match packet.damage_type {
            DamageType::Physical => self.physical_res,
            DamageType::Fire => self.fire_res,
            DamageType::Ice => self.ice_res,
            DamageType::Poison => self.poison_res,
            DamageType::Holy => self.holy_res,
            DamageType::True => 0.0,
        };

        // Учёт пробития брони
        let effective_res = (base_res * (1.0 - packet.armor_penetration)).clamp(0.0, 0.95);
        let mut damage = packet.amount * (1.0 - effective_res);

        // Для физ. урона применяем плоскую броню
        if packet.damage_type == DamageType::Physical {
            let effective_armor = self.flat_armor * (1.0 - packet.armor_penetration);
            damage = (damage - effective_armor).max(1.0);
        }

        damage
    }
}
