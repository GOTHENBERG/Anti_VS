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
            description: "+15% к скорости перезарядки оружия",
        },
        PassiveType::GiantHeart => PassiveDefinition {
            kind,
            name: "Сердце Гиганта",
            description: "+25% к максимальному здоровью и регенерации",
        },
        PassiveType::CooldownTome => PassiveDefinition {
            kind,
            name: "Фолиант Времени",
            description: "-20% от времени отката способностей",
        },
        PassiveType::LightArmor => PassiveDefinition {
            kind,
            name: "Броня Света",
            description: "+8 к плоской броне и +15% к физ. защите",
        },
    }
}
