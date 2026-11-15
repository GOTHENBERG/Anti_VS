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
            if let Ok(mut model) = serde_json::from_str::<BossModel>(&content) {
                let default_w = crate::ai::models::default_weapon_priorities();
                while model.weapon_priorities.len() < default_w.len() {
                    let idx = model.weapon_priorities.len();
                    model.weapon_priorities.push(default_w[idx]);
                }
                let default_p = crate::ai::models::default_passive_priorities();
                while model.passive_priorities.len() < default_p.len() {
                    let idx = model.passive_priorities.len();
                    model.passive_priorities.push(default_p[idx]);
                }
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
            if let Ok(mut model) = serde_json::from_str::<DirectorModel>(&content) {
                let default_s = crate::ai::models::default_squad_weights();
                while model.squad_spawn_weights.len() < default_s.len() {
                    let idx = model.squad_spawn_weights.len();
                    model.squad_spawn_weights.push(default_s[idx]);
                }
                return Some(model);
            }
        }
    }
    None
}
