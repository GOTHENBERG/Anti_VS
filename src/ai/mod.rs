pub mod models;
pub mod neural_net;
pub mod sensors;
pub mod simulation;
pub mod trainer;

use std::path::Path;
use crate::ai::models::{BossModel, DirectorModel};

/// Загружает обученную модель интеллекта босса из models/boss_policy.json, если она существует
pub fn load_trained_boss_model() -> Option<BossModel> {
    let path = Path::new("models/boss_policy.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(model) = serde_json::from_str::<BossModel>(&content) {
                return Some(model);
            }
        }
    }
    None
}

/// Загружает обученную модель тактики директора из models/director_policy.json, если она существует
pub fn load_trained_director_model() -> Option<DirectorModel> {
    let path = Path::new("models/director_policy.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(model) = serde_json::from_str::<DirectorModel>(&content) {
                return Some(model);
            }
        }
    }
    None
}
