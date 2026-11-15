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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    Boss,
    Director,
    Neutral,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resistances {
    pub flat_armor: f32,
    pub physical_res: f32, // 0.0 .. 0.95
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DamagePacket {
    pub source_id: Option<usize>,
    pub target_id: usize,
    pub amount: f32,
    pub damage_type: DamageType,
    pub armor_penetration: f32, // e.g. 0.20 = 20%
}

impl Resistances {
    /// Расчет урона в строгом соответствии с формулами:
    /// Эффективный резист = Базовый резист * (1.0 - Пробитие брони).
    /// Урон после резиста = Базовый урон * (1.0 - Эффективный резист).
    /// Итоговый физический урон = Максимум(1.0, Урон после резиста - Эффективная плоская броня).
    pub fn calculate_damage(&self, packet: &DamagePacket, is_undead: bool) -> f32 {
        if packet.damage_type == DamageType::True {
            return packet.amount;
        }

        let mut incoming_amount = packet.amount;
        // Святой урон наносит +75% бонусного урона нежити
        if packet.damage_type == DamageType::Holy && is_undead {
            incoming_amount *= 1.75;
        }

        let base_res = match packet.damage_type {
            DamageType::Physical => self.physical_res,
            DamageType::Fire => self.fire_res,
            DamageType::Ice => self.ice_res,
            DamageType::Poison => self.poison_res,
            DamageType::Holy => self.holy_res,
            DamageType::True => 0.0,
        };

        // Эффективный резист = Базовый резист * (1.0 - Пробитие брони атакующим)
        let effective_res = (base_res * (1.0 - packet.armor_penetration)).clamp(0.0, 0.95);
        let dmg_after_res = incoming_amount * (1.0 - effective_res);

        // Для физ. урона вычитается эффективная плоская броня
        if packet.damage_type == DamageType::Physical {
            let effective_armor = (self.flat_armor * (1.0 - packet.armor_penetration)).max(0.0);
            (dmg_after_res - effective_armor).max(1.0)
        } else {
            // Огонь, яд, холод и святой игнорируют плоскую броню
            dmg_after_res.max(0.5)
        }
    }
}

/// Статусные эффекты на сущностях (горение, замедление, заморозка, яд)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatusEffects {
    pub burn_timer: f32,
    pub burn_dps: f32,

    pub slow_timer: f32,
    pub slow_factor: f32,

    pub poison_timer: f32,
    pub poison_dps: f32,

    pub freeze_timer: f32,
}

impl StatusEffects {
    pub fn is_slowed(&self) -> bool {
        self.slow_timer > 0.0
    }

    pub fn is_frozen(&self) -> bool {
        self.freeze_timer > 0.0
    }

    pub fn get_speed_multiplier(&self) -> f32 {
        if self.freeze_timer > 0.0 {
            0.0
        } else if self.slow_timer > 0.0 {
            (1.0 - self.slow_factor).clamp(0.2, 1.0)
        } else {
            1.0
        }
    }

    pub fn apply_burn(&mut self, duration: f32, dps: f32) {
        self.burn_timer = self.burn_timer.max(duration);
        self.burn_dps = self.burn_dps.max(dps);
    }

    pub fn apply_slow(&mut self, duration: f32, factor: f32) {
        self.slow_timer = self.slow_timer.max(duration);
        self.slow_factor = self.slow_factor.max(factor);
    }

    pub fn apply_poison(&mut self, duration: f32, dps: f32) {
        self.poison_timer = self.poison_timer.max(duration);
        self.poison_dps = self.poison_dps.max(dps);
    }

    pub fn apply_freeze(&mut self, duration: f32) {
        self.freeze_timer = self.freeze_timer.max(duration);
    }
}
