use crate::ecs::components::inventory::PassiveType;

pub struct PassiveDefinition {
    pub kind: PassiveType,
    pub name: &'static str,
    pub description: &'static str,
}

pub const ALL_LEGENDARY_RELICS: [PassiveType; 21] = [
    PassiveType::RevivalPhylactery,
    PassiveType::ElderMageShard,
    PassiveType::DoomedBlade,
    PassiveType::OccultPrism,
    PassiveType::SolarRelic,
    PassiveType::StaticCore,
    PassiveType::ColossalEye,
    PassiveType::BlessedHeart,
    PassiveType::Pyromaniac,
    PassiveType::SoulHarvester,
    PassiveType::DivinityAura,
    PassiveType::UkuleleResonance,
    PassiveType::FateWeaver,
    PassiveType::CalamityMortar,
    PassiveType::SporeMushroom,
    PassiveType::TwinPrism,
    PassiveType::AbyssalBrazier,
    PassiveType::AbyssalCurseStone,
    PassiveType::MoonDew,
    PassiveType::GoldenSeal,
    PassiveType::TitanBloodArmor,
];

pub fn is_legendary_relic(kind: PassiveType) -> bool {
    matches!(
        kind,
        PassiveType::RevivalPhylactery
            | PassiveType::ElderMageShard
            | PassiveType::DoomedBlade
            | PassiveType::OccultPrism
            | PassiveType::SolarRelic
            | PassiveType::StaticCore
            | PassiveType::ColossalEye
            | PassiveType::BlessedHeart
            | PassiveType::Pyromaniac
            | PassiveType::SoulHarvester
            | PassiveType::DivinityAura
            | PassiveType::UkuleleResonance
            | PassiveType::FateWeaver
            | PassiveType::CalamityMortar
            | PassiveType::SporeMushroom
            | PassiveType::TwinPrism
            | PassiveType::AbyssalBrazier
            | PassiveType::AbyssalCurseStone
            | PassiveType::MoonDew
            | PassiveType::GoldenSeal
            | PassiveType::TitanBloodArmor
    )
}

pub fn get_passive_def(kind: PassiveType) -> PassiveDefinition {
    match kind {
        PassiveType::SpeedGloves => PassiveDefinition {
            kind,
            name: "Перчатка Скорости",
            description: "+15% к скорости атаки оружия. Нужна для Шквала Лезвий.",
        },
        PassiveType::GiantHeart => PassiveDefinition {
            kind,
            name: "Сердце Гиганта",
            description: "+25% к макс. здоровью и регенерации. Нужна для Пекла Преисподней.",
        },
        PassiveType::CooldownTome => PassiveDefinition {
            kind,
            name: "Фолиант Времени",
            description: "-20% от времени отката способностей. Нужен для Луча Смерти.",
        },
        PassiveType::LightArmor => PassiveDefinition {
            kind,
            name: "Броня Света",
            description: "+8 к плоской броне и +15% к физ. защите. Нужна для Бастиона Света.",
        },
        PassiveType::FrostRing => PassiveDefinition {
            kind,
            name: "Кольцо Заморозки",
            description: "+20% к длительности замедления и аура холода. Нужна для Ледниковой Бури.",
        },
        PassiveType::CorrosiveFlask => PassiveDefinition {
            kind,
            name: "Едкий Сосуд",
            description: "+15% к пробитию брони для всех атак. Нужен для Токсичного Мора.",
        },
        PassiveType::CrimsonHeartstone => PassiveDefinition {
            kind,
            name: "Багровый Сердцевик",
            description: "+12% к общему урону и +2.5 HP/сек регенерации. Нужен для Вихревого Секача.",
        },
        PassiveType::EagleScope => PassiveDefinition {
            kind,
            name: "Орлиный Прицел",
            description: "+20% к скорости и дальности полета всех снарядов. Нужен для Звездопадного Залпа.",
        },
        PassiveType::ChronoHourglass => PassiveDefinition {
            kind,
            name: "Песочные Часы",
            description: "+25% к длительности заморозки, горения и яда. Нужен для Гнева Грозы.",
        },
        PassiveType::MagnetSphere => PassiveDefinition {
            kind,
            name: "Сфера Притяжения",
            description: "+45px к радиусу притяжения кристаллов душ. Нужна для Кровавой Жатвы.",
        },
        PassiveType::TeslaCoil => PassiveDefinition {
            kind,
            name: "Катушка Теслы",
            description: "+12% шанс цепной молнии на 3 цели. Нужна для Горизонта Событий.",
        },
        PassiveType::FortuneCoin => PassiveDefinition {
            kind,
            name: "Монета Удачи",
            description: "+12% шанс критического удара (х2 урон). Нужна для Абсолютного Ноля.",
        },
        PassiveType::PrismaticFocus => PassiveDefinition {
            kind,
            name: "Призматический Фокус",
            description: "+15% к радиусу поражения (AoE). Нужен для Инфернального Катаклизма.",
        },
        PassiveType::TitanAegis => PassiveDefinition {
            kind,
            name: "Черный Эгис",
            description: "-2.5 плоского урона от каждого удара. Нужен для Осады Судного Дня.",
        },
        PassiveType::RevivalPhylactery => PassiveDefinition {
            kind,
            name: "Филактерия Возрождения",
            description: "Воскрешение со 100% HP и мощной ударной волной при гибели. Эволюция Бастиона.",
        },
        PassiveType::ElderMageShard => PassiveDefinition {
            kind,
            name: "Осколок Первомага",
            description: "+1 дополнительный рикошет снарядов, -15% к откату и +20% к площади всех атак.",
        },
        PassiveType::DoomedBlade => PassiveDefinition {
            kind,
            name: "Клинок Обречённых",
            description: "+60% к общему наносимому урону, но +15% к получаемому боссом урону.",
        },
        PassiveType::OccultPrism => PassiveDefinition {
            kind,
            name: "Оккультная Призма",
            description: "6% вампиризма от любого урона заклинаний. Эволюция Коллапса Сверхновой.",
        },
        PassiveType::SolarRelic => PassiveDefinition {
            kind,
            name: "Солнечный Венец",
            description: "Аура солнца: 40 святого урона/сек вокруг босса, ослабляет урон врагов на 20%.",
        },
        PassiveType::StaticCore => PassiveDefinition {
            kind,
            name: "Громовой Молот",
            description: "25% шанс вызвать цепную молнию на 6 целей при ударе. Эволюция Гнева Бури.",
        },
        PassiveType::ColossalEye => PassiveDefinition {
            kind,
            name: "Око Колосса",
            description: "Снаряды пронзают насквозь и наносят +100% урона, но темп стрельбы снижен на 20%.",
        },
        PassiveType::BlessedHeart => PassiveDefinition {
            kind,
            name: "Благословенное Сердце",
            description: "+150 к макс. HP, +25% к урону и наделяет абсолютно все снаряды самонаведением.",
        },
        PassiveType::Pyromaniac => PassiveDefinition {
            kind,
            name: "Сердце Пироманта",
            description: "Огонь и взрывы исцеляют босса на 35% от нанесенного урона. Эволюция Армагеддона.",
        },
        PassiveType::SoulHarvester => PassiveDefinition {
            kind,
            name: "Жатва Душ",
            description: "Убийство врагов с шансом 20% выпускает мстительного духа. Эволюция Вопля Банши.",
        },
        PassiveType::DivinityAura => PassiveDefinition {
            kind,
            name: "Аура Божественности",
            description: "Все снаряды излучают сияющий ореол света на 35 AoE урона в полете. Эволюция Собора.",
        },
        PassiveType::UkuleleResonance => PassiveDefinition {
            kind,
            name: "Электрическая Лира",
            description: "25% шанс при попадании выпустить цепочку искр на 4 дополнительные цели.",
        },
        PassiveType::FateWeaver => PassiveDefinition {
            kind,
            name: "Ткач Судьбы",
            description: "Колесо Фортуны: все шансы срабатывания эффектов бросаются дважды в пользу босса.",
        },
        PassiveType::CalamityMortar => PassiveDefinition {
            kind,
            name: "Мортира Бедствия",
            description: "Прямые попадания вызывают вторичную детонацию по площади радиусом 55px.",
        },
        PassiveType::SporeMushroom => PassiveDefinition {
            kind,
            name: "Споровый Мицелий",
            description: "Пускает целебные корни при остановке. Восстанавливает HP даже в гуще врагов и под градом ударов!",
        },
        PassiveType::TwinPrism => PassiveDefinition {
            kind,
            name: "Призма Удвоения",
            description: "+1/+2/+3 дополнительных снаряда/луча/книги ко ВСЕМ способностям босса. Нужна для Искривления Пространства.",
        },
        PassiveType::AbyssalBrazier => PassiveDefinition {
            kind,
            name: "Жаровня Бездны",
            description: "+20%/40%/60% к радиусу и площади (AoE) всех заклинаний, луж, аур и взрывов босса.",
        },
        PassiveType::AbyssalCurseStone => PassiveDefinition {
            kind,
            name: "Проклятый Камень Бездны",
            description: "+35% к получаемому опыту и +25% к наносимому урону босса за счет усиления натиска вражеских полчищ.",
        },
        PassiveType::MoonDew => PassiveDefinition {
            kind,
            name: "Лунная Роса",
            description: "+3.0 HP/сек регенерация и +15% сопротивления холоду, яду и магии. Нужна для Бесконечной Дилатации.",
        },
        PassiveType::GoldenSeal => PassiveDefinition {
            kind,
            name: "Имперская Печать",
            description: "+20% к общему урону, +10% шанс критического удара и +30px к радиусу притяжения кристаллов душ.",
        },
        PassiveType::TitanBloodArmor => PassiveDefinition {
            kind,
            name: "Доспех Кровавого Титана",
            description: "Преобразует потерянное здоровье в несокрушимую плоскую броню (+до 25) и яростный базовый урон (+20%).",
        },
        PassiveType::FrenzyIchor => PassiveDefinition {
            kind,
            name: "Ихор Неистовства",
            description: "Ихор неистовства: повышает регенерацию (до +38 HP/с) и темп атаки оружия (до +60%) тем сильнее, чем ниже здоровье босса.",
        },
        PassiveType::MartyrVanguard => PassiveDefinition {
            kind,
            name: "Авангард Мучеников",
            description: "Командный авангард: ускоряет бег (до +60%) тем сильнее, чем меньше HP у союзников. Убийство любого врага лечит на +1.5 HP.",
        },
    }
}
