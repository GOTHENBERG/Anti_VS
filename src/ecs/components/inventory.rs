use serde::{Deserialize, Serialize};
use crate::ecs::components::combat::DamageType;

pub const MAX_WEAPON_SLOTS: usize = 6;
pub const MAX_PASSIVE_SLOTS: usize = 10; // Расширено до 10 слотов для создания глубоких синергетических билдов!
pub const MAX_WEAPON_LEVEL: u8 = 8;
pub const MAX_PASSIVE_LEVEL: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponType {
    // Базовые оружия и скиллы (20 видов)
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
    SingularityOrb,   // 11. Гравитационная Сфера (Черная дыра)
    ChainFrost,       // 12. Цепной Ледяной Шар (Прыгающий мороз)
    BrimstoneRay,     // 13. Кровавый Гиперлуч (Прожигающий лазер)
    MissileSwarm,     // 14. Рой Микроракет (Самонаводящиеся ракеты)
    ChainLightning,   // 15. Цепная Молния (Static Arc)
    DivineBell,       // 16. Священный Колокол (Resonance Clang)
    SoulPhantoms,     // 17. Голодные Призраки (Ethereal Swarm)
    AegisOrbiters,    // 18. Орбитальные Бастионы (Shield Orbiters)
    ChaosMeteor,      // 19. Хаотический Метеор (Astral Cataclysm)
    VoidRift,         // 20. Разрыв Бездны (Spatial Rift)
    SacredScriptures, // 21. Орбитальный Гримуар (Вращающиеся фолианты)
    FaithBoomerang,   // 22. Клинок Веры (Возвращающийся крестообразный бумеранг)
    CursedChalice,    // 23. Осквернённая Чаша (Священный огонь, разливающийся в лужи)
    VampiricAura,     // 24. Аура Вампиризма (Тёмное кольцо похищения жизни)
    RunicPrism,       // 25. Руническая Призма (Рикошетящие лучи-осколки)
    AbyssalBeam,      // 26. Столп Бездны (Вертикальный луч небесного гнева)
    ChronoStasis,     // 27. Хроно-Стазис (Конус временной заморозки)
    AstralCloak,      // 28. Астральный Саван (Защитный поглощающий кокон)

    // Эволюции (28 эволюций)
    BladeStorm,       // 1. Буря Клинков (Клинок Ветра + Перчатка Скорости)
    Hellfire,         // 2. Адское Пекло (Огненная Аура + Сердце Гиганта)
    DeathRay,         // 3. Луч Смерти (Магический Жезл + Фолиант Времени)
    DivineShield,     // 4. Праведный Щит (Святой Молот + Броня Света)
    BlizzardStorm,    // 5. Ледниковая Буря (Ледяные Осколки + Кольцо Заморозки)
    ToxicPestilence,  // 6. Токсичный Мор (Чумные Сферы + Едкий Сосуд)
    WhirlwindCleaver, // 7. Вихревой Секач (Топор Палача + Кровавый Камень)
    StarfallBarrage,  // 8. Звездопадный Залп (Небесный Лук + Орлиный Прицел)
    ThunderstormWrath,// 9. Гнев Грозы (Грозовая Сфера + Песочные Часы)
    BloodHarvest,     // 10. Кровавая Жатва (Кровавые Кинжалы + Сфера Притяжения)
    EventHorizon,     // 11. Горизонт Событий (Гравитационная Сфера + Катушка Теслы)
    AbsoluteZero,     // 12. Абсолютный Ноль (Цепной Ледяной Шар + Клевер Судьбы)
    InfernalCataclysm,// 13. Инфернальный Катаклизм (Кровавый Гиперлуч + Призматический Фокус)
    DoomsdayBarrage,  // 14. Осада Судного Дня (Рой Микроракет + Черный Эгис)
    StormGodWrath,    // 15. Гнев Буревестника (Цепная Молния + Громовой Молот)
    CathedralAura,    // 16. Великий Благовест (Священный Колокол + Аура Божественности)
    BansheeWail,      // 17. Вопль Банши (Голодные Призраки + Жатва Душ)
    InvincibleBastion,// 18. Несокрушимая Крепость (Орбитальные Бастионы + Эгида Бессмертия)
    ArmageddonStorm,  // 19. Шторм Армагеддона (Хаотический Метеор + Сердце Пироманта)
    SingularityCollapse,// 20. Коллапс Сверхновой (Разрыв Бездны + Оккультная Призма)
    InfernalLiturgy,  // 21. Инфернальная Литургия (Орбитальный Гримуар + Песочные Часы)
    HeavenlyJudgment, // 22. Небесный Вердикт (Клинок Веры + Клевер Судьбы)
    BloodDeluge,      // 23. Кровавый Потоп (Осквернённая Чаша + Призматический Фокус)
    SoulFeast,        // 24. Пиршество Душ (Аура Вампиризма + Кровавый Камень)
    SpatialShatter,   // 25. Искривление Пространства (Руническая Призма + Зеркальный Дупликатор)
    ApocalypseCascade,// 26. Каскад Апокалипсиса (Столп Бездны + Фолиант Времени)
    InfiniteDilation, // 27. Бесконечная Дилатация (Хроно-Стазис + Серебряная Слеза)
    BloodEmperorShroud,// 28. Саван Кровавого Императора (Астральный Саван + Черный Эгис)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassiveType {
    // Базовые пассивки (14 видов)
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
    TeslaCoil,        // 11. Катушка Теслы (+шанс цепной молнии)
    CloverOfFate,     // 12. Клевер Судьбы (+шанс крит. удара)
    PrismaticFocus,   // 13. Призматический Фокус (+радиус поражения AoE)
    TitanAegis,       // 14. Черный Эгис (плоское снижение входящего урона)

    // Легендарные реликвии и кросс-синергетические предметы (21 вид)
    ImmortalAegis,    // 15. Эгида Бессмертия (Воскрешение с 65% HP + ударная волна)
    ArchmageShard,    // 16. Осколок Архимага (+1 рикошет, -15% кд, +20% AoE всем скиллам)
    DivineRapier,     // 17. Священная Рапира (+60% общий урон, но +15% входящий урон)
    OctarineCore,     // 18. Оккультная Призма (22% вампиризм от всех заклинаний)
    SolarRelic,       // 19. Солнечный Венец (40 AoE ожог/сек, враги наносят на 20% меньше урона)
    StaticCore,       // 20. Громовой Молот (25% шанс цепной молнии на 6 целей)
    ColossalEye,      // 21. Око Колосса (-25% темп атаки, +110% урон, +50% размер, 100% пробивание)
    BlessedHeart,     // 22. Благословенное Сердце (+35% урон, +150 HP, самонаведение всех снарядов)
    Pyromaniac,       // 23. Сердце Пироманта (Огонь и взрывы лечат босса на 40% урона)
    SoulHarvester,    // 24. Жатва Душ (Убийства призывают духов-фамильяров)
    DivinityAura,     // 25. Аура Божественности (Снаряды излучают сияющий AoE урон в полете)
    UkuleleResonance, // 26. Электрическая Лира (25% шанс цепных импульсов на 4 врагов)
    FateWeaver,       // 27. Ткач Судьбы (57-листный клевер: все шансы проков бросаются дважды)
    SiegeMortar,      // 28. Осадный Бегемот (Все попадания детонируют в радиусе 55px на 50% урона)
    SporeMushroom,    // 29. Споровый Мицелий (Целебные грибы при остановке)
    DuplicatorRelic,  // 30. Зеркальный Дупликатор (+1/+2/+3 снаряда ко всем скиллам)
    CandelabraOfFury, // 31. Канделябр Ярости (+20%/40%/60% к радиусу AoE атак и луж)
    AbyssalCurseStone,// 32. Проклятый Камень Бездны (+35% XP, +25% урон босса)
    SilverTears,      // 33. Серебряная Слеза (+3.0 HP/сек реген, +15% резисты)
    GoldenSeal,       // 34. Имперская Печать (+20% урон, +10% крит, +30px сбор)
    TitanBloodArmor,  // 35. Доспех Кровавого Титана (Бонус урона и брони от потерянного HP)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeaponSlot {
    pub kind: WeaponType,
    pub level: u8,
    pub cooldown_timer: f32,
    pub base_cooldown: f32,
    pub damage: f32,
    pub is_evolved: bool,
    pub limit_break_level: u32,
    pub bonus_projectiles: u32,
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
    // Запредельное Возвышение (Limit Break после 10-й минуты)
    LimitBreakDamage { weapon_idx: usize, name: String },
    LimitBreakCooldown { weapon_idx: usize, name: String },
    LimitBreakProjectiles { weapon_idx: usize, name: String },
    LimitBreakTitanVigor,     // +150 Max HP, +25% Heal
    LimitBreakAdaptiveArmor,  // +2.0 Flat Armor, +3% Resists
    LimitBreakPredatorSpeed,  // +6% Move Speed
}

/// Источник входящего урона по Боссу
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageSourceKind {
    Melee,
    Ranged,
    Hazard, // Взрывы камикадзе, чумные споры, периодический урон
}

/// Детализация урона по всем типам метаданных 1C/Anti_VS
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DamageTypeBreakdown {
    pub physical: f32,
    pub fire: f32,
    pub ice: f32,
    pub poison: f32,
    pub holy: f32,
    pub true_damage: f32,
}

impl DamageTypeBreakdown {
    pub fn add(&mut self, damage_type: DamageType, amount: f32) {
        match damage_type {
            DamageType::Physical => self.physical += amount,
            DamageType::Fire => self.fire += amount,
            DamageType::Ice => self.ice += amount,
            DamageType::Poison => self.poison += amount,
            DamageType::Holy => self.holy += amount,
            DamageType::True => self.true_damage += amount,
        }
    }

    pub fn decay(&mut self, factor: f32) {
        self.physical *= factor;
        self.fire *= factor;
        self.ice *= factor;
        self.poison *= factor;
        self.holy *= factor;
        self.true_damage *= factor;
    }

    pub fn get(&self, damage_type: DamageType) -> f32 {
        match damage_type {
            DamageType::Physical => self.physical,
            DamageType::Fire => self.fire,
            DamageType::Ice => self.ice,
            DamageType::Poison => self.poison,
            DamageType::Holy => self.holy,
            DamageType::True => self.true_damage,
        }
    }

    pub fn total(&self) -> f32 {
        self.physical + self.fire + self.ice + self.poison + self.holy + self.true_damage
    }
}

/// Полная внутренняя статистика полученного Боссом урона
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BossDamageStats {
    // Недавний скользящий урон (с затуханием, для тактической реакции и сенсоров)
    pub melee_recent: f32,
    pub ranged_recent: f32,
    pub hazard_recent: f32,
    pub type_recent: DamageTypeBreakdown,

    // Накопительная статистика урона за весь матч
    pub melee_total: f32,
    pub ranged_total: f32,
    pub hazard_total: f32,
    pub type_total: DamageTypeBreakdown,

    // Количество полученных попаданий / тиков
    pub melee_hits_count: usize,
    pub ranged_hits_count: usize,
    pub hazard_hits_count: usize,
}

impl BossDamageStats {
    pub fn record_hit(&mut self, amount: f32, damage_type: DamageType, source: DamageSourceKind) {
        match source {
            DamageSourceKind::Melee => {
                self.melee_recent += amount;
                self.melee_total += amount;
                self.melee_hits_count += 1;
            }
            DamageSourceKind::Ranged => {
                self.ranged_recent += amount;
                self.ranged_total += amount;
                self.ranged_hits_count += 1;
            }
            DamageSourceKind::Hazard => {
                self.hazard_recent += amount;
                self.hazard_total += amount;
                self.hazard_hits_count += 1;
            }
        }
        self.type_recent.add(damage_type, amount);
        self.type_total.add(damage_type, amount);
    }

    pub fn decay(&mut self, dt: f32) {
        let decay_factor = (1.0 - (dt * 0.25)).max(0.0);
        self.melee_recent *= decay_factor;
        self.ranged_recent *= decay_factor;
        self.hazard_recent *= decay_factor;
        self.type_recent.decay(decay_factor);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BossInventory {
    pub weapons: Vec<WeaponSlot>,
    pub passives: Vec<PassiveSlot>,
    pub level: u32,
    pub current_xp: f32,
    pub xp_to_next_level: f32,
    pub magnet_radius: f32,

    // Адаптивная телеметрия и полная статистика урона
    pub melee_damage_recent: f32,
    pub ranged_damage_recent: f32,
    pub damage_stats: BossDamageStats,
    pub last_chosen_perk: Option<String>,
    pub perk_announcement_timer: f32,

    // Механика однократного воскрешения от Эгиды Бессмертия
    pub has_used_aegis_revive: bool,
    pub bonus_move_speed_multiplier: f32, // Множитель скорости бега (от SpeedGloves и Возвышения)

    // Защитные механики Астрального Савана / Савана Кровавого Императора
    pub astral_cloak_charges: u8,
    pub astral_cloak_timer: f32,
    // Таймер глобальной дилатации времени (Хроно-Стазис / Бесконечная Дилатация)
    pub infinite_dilation_timer: f32,
    // Угол вращения Орбитального Гримуара
    pub scriptures_orbit_angle: f32,
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
                limit_break_level: 0,
                bonus_projectiles: 0,
            }],
            passives: Vec::new(),
            level: 1,
            current_xp: 0.0,
            xp_to_next_level: 40.0,
            magnet_radius: 120.0,
            melee_damage_recent: 0.0,
            ranged_damage_recent: 0.0,
            damage_stats: BossDamageStats::default(),
            last_chosen_perk: None,
            perk_announcement_timer: 0.0,
            has_used_aegis_revive: false,
            bonus_move_speed_multiplier: 1.0,
            astral_cloak_charges: 3,
            astral_cloak_timer: 0.0,
            infinite_dilation_timer: 10.0,
            scriptures_orbit_angle: 0.0,
        }
    }

    /// Регистрирует входящий урон по Боссу с классификацией источника и типа урона
    pub fn record_damage_taken(&mut self, amount: f32, damage_type: DamageType, source: DamageSourceKind) {
        self.damage_stats.record_hit(amount, damage_type, source);
        self.melee_damage_recent = self.damage_stats.melee_recent;
        self.ranged_damage_recent = self.damage_stats.ranged_recent;
    }

    /// Обновляет временное затухание недавнего урона
    pub fn update_damage_decay(&mut self, dt: f32) {
        self.damage_stats.decay(dt);
        self.melee_damage_recent = self.damage_stats.melee_recent;
        self.ranged_damage_recent = self.damage_stats.ranged_recent;
    }

    pub fn add_xp(&mut self, amount: f32) -> usize {
        self.current_xp += amount;
        let mut level_ups = 0;
        while self.current_xp >= self.xp_to_next_level {
            self.current_xp -= self.xp_to_next_level;
            self.level += 1;
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
            WeaponType::SingularityOrb if self.has_passive(PassiveType::TeslaCoil) => {
                Some(WeaponType::EventHorizon)
            }
            WeaponType::ChainFrost if self.has_passive(PassiveType::CloverOfFate) => {
                Some(WeaponType::AbsoluteZero)
            }
            WeaponType::BrimstoneRay if self.has_passive(PassiveType::PrismaticFocus) => {
                Some(WeaponType::InfernalCataclysm)
            }
            WeaponType::MissileSwarm if self.has_passive(PassiveType::TitanAegis) => {
                Some(WeaponType::DoomsdayBarrage)
            }
            WeaponType::ChainLightning if self.has_passive(PassiveType::StaticCore) => {
                Some(WeaponType::StormGodWrath)
            }
            WeaponType::DivineBell if self.has_passive(PassiveType::DivinityAura) => {
                Some(WeaponType::CathedralAura)
            }
            WeaponType::SoulPhantoms if self.has_passive(PassiveType::SoulHarvester) => {
                Some(WeaponType::BansheeWail)
            }
            WeaponType::AegisOrbiters if self.has_passive(PassiveType::ImmortalAegis) => {
                Some(WeaponType::InvincibleBastion)
            }
            WeaponType::ChaosMeteor if self.has_passive(PassiveType::Pyromaniac) => {
                Some(WeaponType::ArmageddonStorm)
            }
            WeaponType::VoidRift if self.has_passive(PassiveType::OctarineCore) => {
                Some(WeaponType::SingularityCollapse)
            }
            WeaponType::SacredScriptures if self.has_passive(PassiveType::ChronoHourglass) => {
                Some(WeaponType::InfernalLiturgy)
            }
            WeaponType::FaithBoomerang if self.has_passive(PassiveType::CloverOfFate) => {
                Some(WeaponType::HeavenlyJudgment)
            }
            WeaponType::CursedChalice if self.has_passive(PassiveType::PrismaticFocus) => {
                Some(WeaponType::BloodDeluge)
            }
            WeaponType::VampiricAura if self.has_passive(PassiveType::BloodStone) => {
                Some(WeaponType::SoulFeast)
            }
            WeaponType::RunicPrism if self.has_passive(PassiveType::DuplicatorRelic) => {
                Some(WeaponType::SpatialShatter)
            }
            WeaponType::AbyssalBeam if self.has_passive(PassiveType::CooldownTome) => {
                Some(WeaponType::ApocalypseCascade)
            }
            WeaponType::ChronoStasis if self.has_passive(PassiveType::SilverTears) => {
                Some(WeaponType::InfiniteDilation)
            }
            WeaponType::AstralCloak if self.has_passive(PassiveType::TitanAegis) => {
                Some(WeaponType::BloodEmperorShroud)
            }
            _ => None,
        }
    }

    /// Возвращает требуемую пассивку для эволюции оружия
    pub fn get_evolution_passive_for_weapon(kind: WeaponType) -> Option<PassiveType> {
        match kind {
            WeaponType::WindBlade => Some(PassiveType::SpeedGloves),
            WeaponType::FireAura => Some(PassiveType::GiantHeart),
            WeaponType::MagicWand => Some(PassiveType::CooldownTome),
            WeaponType::HolyHammer => Some(PassiveType::LightArmor),
            WeaponType::IceShards => Some(PassiveType::FrostRing),
            WeaponType::PoisonOrbs => Some(PassiveType::CorrosiveFlask),
            WeaponType::ExecutionerAxe => Some(PassiveType::BloodStone),
            WeaponType::CelestialBow => Some(PassiveType::EagleScope),
            WeaponType::ThunderOrb => Some(PassiveType::ChronoHourglass),
            WeaponType::BloodDaggers => Some(PassiveType::MagnetSphere),
            WeaponType::SingularityOrb => Some(PassiveType::TeslaCoil),
            WeaponType::ChainFrost => Some(PassiveType::CloverOfFate),
            WeaponType::BrimstoneRay => Some(PassiveType::PrismaticFocus),
            WeaponType::MissileSwarm => Some(PassiveType::TitanAegis),
            WeaponType::ChainLightning => Some(PassiveType::StaticCore),
            WeaponType::DivineBell => Some(PassiveType::DivinityAura),
            WeaponType::SoulPhantoms => Some(PassiveType::SoulHarvester),
            WeaponType::AegisOrbiters => Some(PassiveType::ImmortalAegis),
            WeaponType::ChaosMeteor => Some(PassiveType::Pyromaniac),
            WeaponType::VoidRift => Some(PassiveType::OctarineCore),
            WeaponType::SacredScriptures => Some(PassiveType::ChronoHourglass),
            WeaponType::FaithBoomerang => Some(PassiveType::CloverOfFate),
            WeaponType::CursedChalice => Some(PassiveType::PrismaticFocus),
            WeaponType::VampiricAura => Some(PassiveType::BloodStone),
            WeaponType::RunicPrism => Some(PassiveType::DuplicatorRelic),
            WeaponType::AbyssalBeam => Some(PassiveType::CooldownTome),
            WeaponType::ChronoStasis => Some(PassiveType::SilverTears),
            WeaponType::AstralCloak => Some(PassiveType::TitanAegis),
            _ => None,
        }
    }

    pub fn get_duplicator_projectiles(&self) -> usize {
        self.get_passive_level(PassiveType::DuplicatorRelic) as usize
    }

    pub fn get_candelabra_area_multiplier(&self) -> f32 {
        1.0 + (self.get_passive_level(PassiveType::CandelabraOfFury) as f32 * 0.20)
    }

    pub fn get_curse_stone_xp_multiplier(&self) -> f32 {
        1.0 + (self.get_passive_level(PassiveType::AbyssalCurseStone) as f32 * 0.35)
    }

    pub fn get_silver_tears_regen(&self) -> f32 {
        self.get_passive_level(PassiveType::SilverTears) as f32 * 3.0
    }

    /// Проверяет, является ли пассивка ключом к эволюции для указанного оружия
    pub fn is_evolution_partner(weapon: WeaponType, passive: PassiveType) -> bool {
        Self::get_evolution_passive_for_weapon(weapon) == Some(passive)
    }

    /// Ткач Судьбы (57-листный клевер): перебрасывает неудачные шансы
    pub fn roll_luck(&self, chance: f32) -> bool {
        let has_clover = self.has_passive(PassiveType::FateWeaver);
        let roll1 = crate::core::rand::gen_range(0.0, 1.0) < chance;
        if roll1 || !has_clover {
            roll1
        } else {
            crate::core::rand::gen_range(0.0, 1.0) < chance
        }
    }

    /// Шанс удара цепной молнией от TeslaCoil / StaticCore
    pub fn get_tesla_chain_chance(&self) -> f32 {
        let lvl = self.get_passive_level(PassiveType::TeslaCoil);
        let static_core = if self.has_passive(PassiveType::StaticCore) { 0.25 } else { 0.0 };
        ((lvl as f32 * 0.12) + static_core).min(0.75)
    }

    /// Шанс критического удара от CloverOfFate (Клевер Судьбы) и GoldenSeal
    pub fn get_crit_chance(&self) -> f32 {
        let lvl = self.get_passive_level(PassiveType::CloverOfFate);
        let mut chance = (lvl as f32 * 0.12).min(0.60);
        if self.has_passive(PassiveType::GoldenSeal) {
            chance += 0.10;
        }
        chance.min(0.85)
    }

    /// Множитель размера области поражения AoE от PrismaticFocus, CandelabraOfFury и ArchmageShard
    pub fn get_aoe_multiplier(&self) -> f32 {
        let lvl = self.get_passive_level(PassiveType::PrismaticFocus);
        let shard_bonus = if self.has_passive(PassiveType::ArchmageShard) { 0.20 } else { 0.0 };
        let candelabra_bonus = self.get_passive_level(PassiveType::CandelabraOfFury) as f32 * 0.20;
        1.0 + (lvl as f32 * 0.15) + shard_bonus + candelabra_bonus
    }

    /// Плоское снижение любого входящего урона от TitanAegis и TitanBloodArmor
    pub fn get_flat_damage_reduction(&self) -> f32 {
        let lvl = self.get_passive_level(PassiveType::TitanAegis);
        let titan_armor = self.get_passive_level(PassiveType::TitanBloodArmor) as f32 * 3.5;
        (lvl as f32 * 2.5) + titan_armor
    }

    /// Множитель урона от BloodStone, DivineRapier, ColossalEye, BlessedHeart, AbyssalCurseStone, GoldenSeal
    pub fn get_damage_multiplier(&self) -> f32 {
        let blood_stone = self.get_passive_level(PassiveType::BloodStone);
        let mut mult = 1.0 + (blood_stone as f32 * 0.12);
        if self.has_passive(PassiveType::DivineRapier) {
            mult += 0.60; // +60% от Священной Рапиры
        }
        if self.has_passive(PassiveType::ColossalEye) {
            mult += 1.10; // +110% от Ока Колосса
        }
        if self.has_passive(PassiveType::BlessedHeart) {
            mult += 0.35; // +35% от Благословенного Сердца
        }
        if self.has_passive(PassiveType::AbyssalCurseStone) {
            mult += 0.25; // +25% от Проклятого Камня
        }
        if self.has_passive(PassiveType::GoldenSeal) {
            mult += 0.20; // +20% от Имперской Печати
        }
        mult
    }

    /// Множитель скорости полета снарядов (EagleScope)
    pub fn get_projectile_speed_multiplier(&self) -> f32 {
        let eagle_scope = self.get_passive_level(PassiveType::EagleScope);
        1.0 + (eagle_scope as f32 * 0.20)
    }

    /// Регенерация HP в секунду от BloodStone, GiantHeart и SilverTears
    pub fn get_hp_regen(&self) -> f32 {
        let blood_stone = self.get_passive_level(PassiveType::BloodStone);
        let giant_heart = self.get_passive_level(PassiveType::GiantHeart);
        let silver_tears = self.get_passive_level(PassiveType::SilverTears);
        (blood_stone as f32 * 2.5) + (giant_heart as f32 * 3.0) + (silver_tears as f32 * 3.0)
    }

    /// Вампиризм от Оккультной Призмы (Octarine Core)
    pub fn get_spell_lifesteal_ratio(&self) -> f32 {
        if self.has_passive(PassiveType::OctarineCore) {
            0.22 // 22% исцеления от любого магического и стихийного урона
        } else {
            0.0
        }
    }

    /// Эффективный радиус сбора опыта от MagnetSphere и GoldenSeal
    pub fn get_effective_magnet_radius(&self) -> f32 {
        let magnet_sphere = self.get_passive_level(PassiveType::MagnetSphere);
        let seal_bonus = if self.has_passive(PassiveType::GoldenSeal) { 35.0 } else { 0.0 };
        self.magnet_radius + (magnet_sphere as f32 * 45.0) + seal_bonus
    }

    /// Множитель скорости перезарядки от пассивок
    pub fn get_cooldown_multiplier(&self) -> f32 {
        let mut mult = 1.0;
        let speed_gloves = self.get_passive_level(PassiveType::SpeedGloves);
        if speed_gloves > 0 {
            mult *= 1.0 - (speed_gloves as f32 * 0.08);
        }
        let tome = self.get_passive_level(PassiveType::CooldownTome);
        if tome > 0 {
            mult *= 1.0 - (tome as f32 * 0.07);
        }
        if self.has_passive(PassiveType::ArchmageShard) {
            mult *= 0.85; // -15% кд от Осколка Архимага
        }
        // Око Колосса снижает скорость атаки на 25% ради колоссального урона
        if self.has_passive(PassiveType::ColossalEye) {
            mult *= 1.25;
        }
        mult.clamp(0.25, 2.0)
    }

    /// Пробитие брони от пассивки CorrosiveFlask
    pub fn get_armor_penetration(&self) -> f32 {
        let flask = self.get_passive_level(PassiveType::CorrosiveFlask);
        (flask as f32 * 0.10).min(0.50)
    }

    /// Эффективная скорость перемещения босса с учетом пассивок и модификаторов
    pub fn get_movement_speed(&self, base_speed: f32) -> f32 {
        let mut mult = self.bonus_move_speed_multiplier;
        let speed_gloves = self.get_passive_level(PassiveType::SpeedGloves);
        if speed_gloves > 0 {
            mult += speed_gloves as f32 * 0.04; // +4% к скорости бега за уровень
        }
        base_speed * mult
    }
}
