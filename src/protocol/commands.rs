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
    /// Registers a remote or local co-op Boss actor.
    JoinBoss {
        player_id: u8,
        name: String,
        skin: u8,
    },
    /// Latest held controls for one co-op Boss player; host applies them on its fixed tick.
    BossInput {
        player_id: u8,
        move_dir: NetVec2,
        aim_dir: NetVec2,
        dash_pressed: bool,
    },
    /// Host stopped receiving a player's packets; switch the actor to AI control.
    DisconnectBoss {
        player_id: u8,
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
    /// Переключение глобальной паузы матча (от любого игрока: Хоста или Клиента)
    TogglePause {
        player_id: u8,
        paused: bool,
    },
    /// Выбор перка/улучшения при Level-Up Босса (не блокирует игру для других)
    SelectPerk {
        player_id: u8,
        choice_index: u8,
    },
    /// Перезапуск игры
    RestartGame,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_pause_command_serde() {
        let cmd = Command::TogglePause { player_id: 2, paused: true };
        let serialized = serde_json::to_string(&cmd).expect("Serialize Command::TogglePause");
        let deserialized: Command = serde_json::from_str(&serialized).expect("Deserialize Command::TogglePause");
        match deserialized {
            Command::TogglePause { player_id, paused } => {
                assert_eq!(player_id, 2);
                assert!(paused);
            }
            _ => panic!("Expected TogglePause command"),
        }
    }

    #[test]
    fn test_select_perk_command_serde() {
        let cmd = Command::SelectPerk { player_id: 3, choice_index: 1 };
        let serialized = serde_json::to_string(&cmd).expect("Serialize Command::SelectPerk");
        let deserialized: Command = serde_json::from_str(&serialized).expect("Deserialize Command::SelectPerk");
        match deserialized {
            Command::SelectPerk { player_id, choice_index } => {
                assert_eq!(player_id, 3);
                assert_eq!(choice_index, 1);
            }
            _ => panic!("Expected SelectPerk command"),
        }
    }
}
