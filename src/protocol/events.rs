use crate::protocol::commands::NetVec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    EntityDamaged {
        target_id: usize,
        amount: f32,
        is_crit: bool,
    },
    EntityDied {
        entity_id: usize,
        pos: NetVec2,
        is_boss: bool,
    },
    BossLevelUp {
        new_level: u32,
    },
    DirectorIncomeTicked {
        director_id: u8,
        added_points: f32,
        current_points: f32,
    },
    GameOver {
        boss_won: bool,
    },
}
