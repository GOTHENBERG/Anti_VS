pub const MAX_WEAPON_SLOTS: usize = 6;
pub const MAX_PASSIVE_SLOTS: usize = 6;
pub const MAX_WEAPON_LEVEL: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponType {
    WindBlade,    // Меч ветра (базовый)
    FireAura,     // Огненная аура
    MagicWand,    // Магический жезл
    HolyHammer,   // Святой молот
    BladeStorm,   // ЭВОЛЮЦИЯ: Буря клинков
    InfernalAura, // ЭВОЛЮЦИЯ: Адское пекло
    DeathRay,     // ЭВОЛЮЦИЯ: Луч смерти
    DivineShield, // ЭВОЛЮЦИЯ: Праведный щит
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassiveType {
    SpeedGloves, // Скорость атаки
    GiantHeart,  // ХП и регенерация
    CooldownTome,// Снижение перезарядки
    LightArmor,  // Броня и резисты
}

#[derive(Clone, Debug)]
pub struct WeaponSlot {
    pub kind: WeaponType,
    pub level: u8,
    pub cooldown_timer: f32,
    pub base_cooldown: f32,
    pub damage: f32,
    pub is_evolved: bool,
}

#[derive(Clone, Debug)]
pub struct PassiveSlot {
    pub kind: PassiveType,
    pub level: u8,
}

#[derive(Clone, Debug)]
pub struct BossInventory {
    pub weapons: Vec<WeaponSlot>,
    pub passives: Vec<PassiveSlot>,
    pub level: u32,
    pub current_xp: f32,
    pub xp_to_next_level: f32,
    pub magnet_radius: f32,
}

impl BossInventory {
    pub fn new() -> Self {
        Self {
            weapons: vec![WeaponSlot {
                kind: WeaponType::WindBlade,
                level: 1,
                cooldown_timer: 0.0,
                base_cooldown: 1.2,
                damage: 25.0,
                is_evolved: false,
            }],
            passives: Vec::new(),
            level: 1,
            current_xp: 0.0,
            xp_to_next_level: 50.0,
            magnet_radius: 120.0,
        }
    }

    pub fn add_xp(&mut self, amount: f32) -> bool {
        self.current_xp += amount;
        if self.current_xp >= self.xp_to_next_level {
            self.current_xp -= self.xp_to_next_level;
            self.level += 1;
            self.xp_to_next_level = (self.xp_to_next_level * 1.25).floor();
            true // Level Up!
        } else {
            false
        }
    }

    /// Проверка возможности эволюции оружия
    pub fn can_evolve(&self, weapon_idx: usize) -> Option<WeaponType> {
        let weapon = self.weapons.get(weapon_idx)?;
        if weapon.level < MAX_WEAPON_LEVEL || weapon.is_evolved {
            return None;
        }

        // Проверяем наличие парной пассивки
        match weapon.kind {
            WeaponType::WindBlade if self.has_passive(PassiveType::SpeedGloves) => {
                Some(WeaponType::BladeStorm)
            }
            WeaponType::FireAura if self.has_passive(PassiveType::GiantHeart) => {
                Some(WeaponType::InfernalAura)
            }
            WeaponType::MagicWand if self.has_passive(PassiveType::CooldownTome) => {
                Some(WeaponType::DeathRay)
            }
            WeaponType::HolyHammer if self.has_passive(PassiveType::LightArmor) => {
                Some(WeaponType::DivineShield)
            }
            _ => None,
        }
    }

    pub fn has_passive(&self, kind: PassiveType) -> bool {
        self.passives.iter().any(|p| p.kind == kind)
    }
}
