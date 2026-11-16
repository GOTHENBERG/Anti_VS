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
    /// Призыв отряда мобов Директором (клавиши 1-8)
    SpawnSquad {
        director_id: u8,
        tier: u8,
    },
    /// Призыв отрядов на все доступные души (Shift + 1-8)
    SpawnSquadMax {
        director_id: u8,
        tier: u8,
    },
    /// Улучшение грейда мобов Директором (клавиши F, G, H)
    UpgradeDirectorGrade {
        director_id: u8,
        grade_type: u8, // 0: Fury, 1: Carapace, 2: Pestilence
    },
    /// Улучшение грейда на все деньги (Shift + F, G, H)
    UpgradeDirectorGradeMax {
        director_id: u8,
        grade_type: u8,
    },
    /// Движение Босса (локальный ИИ или второй игрок в мультиплеере)
    BossMove {
        direction: NetVec2,
    },
    /// Использование экстренного рывка Босса
    BossDash {
        target_dir: NetVec2,
    },
    /// Размещение разрушаемой структуры (Алтарь Крови)
    PlaceStructure {
        director_id: u8,
        structure_type: u8,
        pos: NetVec2,
    },
    /// Переключение страницы мобов Директора (0: Тиры 1-8, 1: Тиры 9-16)
    SetMobPage {
        director_id: u8,
        page: u8,
    },
    /// Перезапуск игры
    RestartGame,
}
