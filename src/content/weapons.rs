use crate::ecs::components::inventory::WeaponType;

pub struct WeaponDefinition {
    pub kind: WeaponType,
    pub name: &'static str,
    pub base_damage: f32,
    pub base_cooldown: f32,
}

pub fn get_weapon_def(kind: WeaponType) -> WeaponDefinition {
    match kind {
        WeaponType::WindBlade => WeaponDefinition {
            kind,
            name: "Клинок Ветра",
            base_damage: 25.0,
            base_cooldown: 1.2,
        },
        WeaponType::BladeStorm => WeaponDefinition {
            kind,
            name: "Буря Клинков (Эволюция)",
            base_damage: 55.0,
            base_cooldown: 0.5,
        },
        WeaponType::MagicWand => WeaponDefinition {
            kind,
            name: "Магический Жезл",
            base_damage: 35.0,
            base_cooldown: 0.8,
        },
        WeaponType::DeathRay => WeaponDefinition {
            kind,
            name: "Луч Смерти (Эволюция)",
            base_damage: 90.0,
            base_cooldown: 0.3,
        },
        _ => WeaponDefinition {
            kind,
            name: "Оружие",
            base_damage: 20.0,
            base_cooldown: 1.0,
        },
    }
}
