use serde::{Deserialize, Serialize};

pub const MAX_WEAPON_SLOTS: usize = 6;
pub const MAX_PASSIVE_SLOTS: usize = 6;
pub const MAX_WEAPON_LEVEL: u8 = 8;
pub const MAX_PASSIVE_LEVEL: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponType {
    // Базовые оружия
    WindBlade,        // 1. Клинок Ветра
    FireAura,         // 2. Огненная Аура
    MagicWand,        // 3. Магический Жезл
    HolyHammer,       // 4. Святой Молот
    IceShards,        // 5. Ледяные Осколки
    PoisonOrbs,       // 6. Чумные Сферы
    ExecutionerAxe,   // 7. Топор Палача
    CelestialBow,     // 8. Небесный Лук
    ThunderOrb,       // 9. Грозовая Сфера
    BloodDaggers,     // 10. Кровавые Кинжалы

    // Эволюции (10 эволюций)
    BladeStorm,       // Эволюция: Буря Клинков (Клинок Ветра + Перчатка Скорости)
    Hellfire,         // Эволюция: Адское Пекло (Огненная Аура + Сердце Гиганта)
    DeathRay,         // Эволюция: Луч Смерти (Магический Жезл + Фолиант Времени)
    DivineShield,     // Эволюция: Праведный Щит (Святой Молот + Броня Света)
    BlizzardStorm,    // Эволюция: Ледниковая Буря (Ледяные Осколки + Кольцо Заморозки)
    ToxicPestilence,  // Эволюция: Токсичный Мор (Чумные Сферы + Едкий Сосуд)
    WhirlwindCleaver, // Эволюция: Вихревой Секач (Топор Палача + Кровавый Камень)
    StarfallBarrage,  // Эволюция: Звездопадный Залп (Небесный Лук + Орлиный Прицел)
    ThunderstormWrath,// Эволюция: Гнев Грозы (Грозовая Сфера + Песочные Часы)
    BloodHarvest,     // Эволюция: Кровавая Жатва (Кровавые Кинжалы + Сфера Притяжения)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassiveType {
    SpeedGloves,      // 1. Перчатка Скорости (+скорость атаки)
    GiantHeart,       // 2. Сердце Гиганта (+макс ХП и регенерация)
    CooldownTome,     // 3. Фолиант Времени (-кд всех способностей)
    LightArmor,       // 4. Броня Света (+плоская броня и % физ. резист)
    FrostRing,        // 5. Кольцо Заморозки (+радиус и сила замедления)
    CorrosiveFlask,   // 6. Едкий Сосуд (+пробитие брони атакующим)
    BloodStone,       // 7. Кровавый Камень (+урон и регенерация)
    EagleScope,       // 8. Орлиный Прицел (+скорость полета снарядов)
    ChronoHourglass,  // 9. Песочные Часы (+длительность эффектов)
    MagnetSphere,     // 10. Сфера Притяжения (+радиус сбора душ)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaponSlot {
    pub kind: WeaponType,
    pub level: u8,
    pub cooldown_timer: f32,
    pub base_cooldown: f32,
    pub damage: f32,
    pub is_evolved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PassiveSlot {
    pub kind: PassiveType,
    pub level: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PerkChoice {
    UpgradeWeapon(WeaponType),
    UpgradePassive(PassiveType),
    EvolveWeapon { from: WeaponType, to: WeaponType },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BossInventory {
    pub weapons: Vec<WeaponSlot>,
    pub passives: Vec<PassiveSlot>,
    pub level: u32,
    pub current_xp: f32,
    pub xp_to_next_level: f32,
    pub magnet_radius: f32,

    // Адаптивная телеметрия для ИИ выбора перков
    pub melee_damage_recent: f32,
    pub ranged_damage_recent: f32,
    pub last_chosen_perk: Option<String>,
    pub perk_announcement_timer: f32,
}

impl BossInventory {
    pub fn new() -> Self {
        Self {
            weapons: vec![WeaponSlot {
                kind: WeaponType::WindBlade,
                level: 1,
                cooldown_timer: 0.0,
                base_cooldown: 1.1,
                damage: 28.0,
                is_evolved: false,
            }],
            passives: Vec::new(),
            level: 1,
            current_xp: 0.0,
            xp_to_next_level: 40.0,
            magnet_radius: 120.0,
            melee_damage_recent: 0.0,
            ranged_damage_recent: 0.0,
            last_chosen_perk: None,
            perk_announcement_timer: 0.0,
        }
    }

    pub fn add_xp(&mut self, amount: f32) -> usize {
        self.current_xp += amount;
        let mut level_ups = 0;
        while self.current_xp >= self.xp_to_next_level {
            self.current_xp -= self.xp_to_next_level;
            self.level += 1;
            // Умеренный рост кривой опыта для динамичного сноуболла босса
            self.xp_to_next_level = (self.xp_to_next_level * 1.20 + 10.0).floor();
            level_ups += 1;
        }
        level_ups
    }

    pub fn has_weapon(&self, kind: WeaponType) -> bool {
        self.weapons.iter().any(|w| w.kind == kind)
    }

    pub fn has_passive(&self, kind: PassiveType) -> bool {
        self.passives.iter().any(|p| p.kind == kind)
    }

    pub fn get_passive_level(&self, kind: PassiveType) -> u8 {
        self.passives.iter().find(|p| p.kind == kind).map(|p| p.level).unwrap_or(0)
    }

    /// Проверка эволюции: Оружие 8 уровня + Связанная пассивка 1 уровня и выше
    pub fn can_evolve(&self, weapon_idx: usize) -> Option<WeaponType> {
        let weapon = self.weapons.get(weapon_idx)?;
        if weapon.level < MAX_WEAPON_LEVEL || weapon.is_evolved {
            return None;
        }

        match weapon.kind {
            WeaponType::WindBlade if self.has_passive(PassiveType::SpeedGloves) => {
                Some(WeaponType::BladeStorm)
            }
            WeaponType::FireAura if self.has_passive(PassiveType::GiantHeart) => {
                Some(WeaponType::Hellfire)
            }
            WeaponType::MagicWand if self.has_passive(PassiveType::CooldownTome) => {
                Some(WeaponType::DeathRay)
            }
            WeaponType::HolyHammer if self.has_passive(PassiveType::LightArmor) => {
                Some(WeaponType::DivineShield)
            }
            WeaponType::IceShards if self.has_passive(PassiveType::FrostRing) => {
                Some(WeaponType::BlizzardStorm)
            }
            WeaponType::PoisonOrbs if self.has_passive(PassiveType::CorrosiveFlask) => {
                Some(WeaponType::ToxicPestilence)
            }
            WeaponType::ExecutionerAxe if self.has_passive(PassiveType::BloodStone) => {
                Some(WeaponType::WhirlwindCleaver)
            }
            WeaponType::CelestialBow if self.has_passive(PassiveType::EagleScope) => {
                Some(WeaponType::StarfallBarrage)
            }
            WeaponType::ThunderOrb if self.has_passive(PassiveType::ChronoHourglass) => {
                Some(WeaponType::ThunderstormWrath)
            }
            WeaponType::BloodDaggers if self.has_passive(PassiveType::MagnetSphere) => {
                Some(WeaponType::BloodHarvest)
            }
            _ => None,
        }
    }

    /// Множитель урона от пассивок (BloodStone)
    pub fn get_damage_multiplier(&self) -> f32 {
        let blood_stone = self.get_passive_level(PassiveType::BloodStone);
        1.0 + (blood_stone as f32 * 0.12) // +12% за уровень
    }

    /// Множитель скорости полета снарядов (EagleScope)
    pub fn get_projectile_speed_multiplier(&self) -> f32 {
        let eagle_scope = self.get_passive_level(PassiveType::EagleScope);
        1.0 + (eagle_scope as f32 * 0.20) // +20% за уровень
    }

    /// Регенерация HP в секунду от BloodStone
    pub fn get_hp_regen(&self) -> f32 {
        let blood_stone = self.get_passive_level(PassiveType::BloodStone);
        blood_stone as f32 * 2.5 // +2.5 HP/сек за уровень
    }

    /// Эффективный радиус сбора опыта от MagnetSphere
    pub fn get_effective_magnet_radius(&self) -> f32 {
        let magnet_sphere = self.get_passive_level(PassiveType::MagnetSphere);
        self.magnet_radius + (magnet_sphere as f32 * 45.0)
    }

    /// Множитель скорости перезарядки от пассивок
    pub fn get_cooldown_multiplier(&self) -> f32 {
        let mut mult = 1.0;
        let speed_gloves = self.get_passive_level(PassiveType::SpeedGloves);
        if speed_gloves > 0 {
            mult *= 1.0 - (speed_gloves as f32 * 0.08); // до -40%
        }
        let tome = self.get_passive_level(PassiveType::CooldownTome);
        if tome > 0 {
            mult *= 1.0 - (tome as f32 * 0.07); // до -35%
        }
        mult.max(0.3)
    }

    /// Пробитие брони от пассивки CorrosiveFlask
    pub fn get_armor_penetration(&self) -> f32 {
        let flask = self.get_passive_level(PassiveType::CorrosiveFlask);
        (flask as f32 * 0.10).min(0.50) // 10% за уровень, до 50%
    }
}
