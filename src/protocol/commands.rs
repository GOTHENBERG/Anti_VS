use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetVec2 {
    pub x: f32,
    pub y: f32,
}

impl From<macroquad::prelude::Vec2> for NetVec2 {
    fn from(v: macroquad::prelude::Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<NetVec2> for macroquad::prelude::Vec2 {
    fn from(v: NetVec2) -> Self {
        Self::new(v.x, v.y)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Призыв отряда мобов Директором
    SpawnSquad {
        director_id: u8,
        tier: u8,
    },
    /// Движение Босса (приходит от ИИ или живого игрока в мультиплеере)
    BossMove {
        direction: NetVec2,
    },
    /// Использование экстренного рывка Босса
    BossDash {
        target_dir: NetVec2,
    },
    /// Выбор карты прокачки при Level-Up
    BossSelectPerk {
        choice_index: usize,
    },
    /// Создание разрушаемого объекта Директором (алтарь, тотем, баррикада)
    PlaceStructure {
        director_id: u8,
        structure_type: u8,
        pos: NetVec2,
    },
}
