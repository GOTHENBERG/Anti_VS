use crate::ecs::components::inventory::PassiveType;

pub struct PassiveDefinition {
    pub kind: PassiveType,
    pub name: &'static str,
    pub description: &'static str,
}

pub fn get_passive_def(kind: PassiveType) -> PassiveDefinition {
    match kind {
        PassiveType::SpeedGloves => PassiveDefinition {
            kind,
            name: "Перчатка Скорости",
            description: "+15% к скорости атаки оружия. Нужна для Бури Клинков.",
        },
        PassiveType::GiantHeart => PassiveDefinition {
            kind,
            name: "Сердце Гиганта",
            description: "+25% к макс. здоровью и регенерации. Нужна для Адского Пекла.",
        },
        PassiveType::CooldownTome => PassiveDefinition {
            kind,
            name: "Фолиант Времени",
            description: "-20% от времени отката способностей. Нужен для Луча Смерти.",
        },
        PassiveType::LightArmor => PassiveDefinition {
            kind,
            name: "Броня Света",
            description: "+8 к плоской броне и +15% к физ. защите. Нужна для Праведного Щита.",
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
        PassiveType::BloodStone => PassiveDefinition {
            kind,
            name: "Кровавый Камень",
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
    }
}
