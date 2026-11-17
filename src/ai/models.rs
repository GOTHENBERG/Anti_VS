use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BossModel {
    pub version: String,
    pub generation: u32,
    pub fitness: f32,
    pub avg_survival_time: f32,
    pub total_synapses: usize,

    // Полносвязная глубокая нейросеть (36 сенсоров -> 32 -> 24 -> 8 выходов)
    pub neural_net: Option<crate::ai::neural_net::NeuralNet>,

    // Поведенческие веса потенциальных полей
    pub mob_repulsion_weight: f32,      // Сила отталкивания от мобов (300.0 - 600.0)
    pub gem_attraction_weight: f32,     // Сила притяжения к кристаллам душ (120.0 - 380.0)
    pub altar_siege_aggression: f32,    // Агрессия штурма вражеских Алтарей Крови (150.0 - 400.0)
    pub sniper_hunt_aggression: f32,    // Агрессия фланкирования культистов-стрелков (180.0 - 450.0)
    pub wall_repel_weight: f32,         // Отталкивание от стен арены (400.0 - 750.0)

    // Боевые триггеры
    pub dash_danger_threshold: usize,   // Число мобов в радиусе 90px для рывка (8 - 18)
    pub retreat_hp_threshold: f32,      // Порог здоровья для панического отступления (0.20 - 0.45)
    pub sniper_engage_min_count: usize, // Сколько стрелков триггерят анти-снайперский натиск (1 - 4)

    // Приоритеты выбора оружия при Level-Up (10 оружий)
    #[serde(default = "default_weapon_priorities")]
    pub weapon_priorities: Vec<f32>,

    // Приоритеты выбора пассивок при Level-Up (28 пассивок и реликвий)
    #[serde(default = "default_passive_priorities")]
    pub passive_priorities: Vec<f32>,

    // Генетические веса 6 имба-архетипов билдостроения:
    // [0] Инфернальный Пиромант (Огонь, Пламя, Сердце Пироманта)
    // [1] Сверхпроводящий Громовержец (Молния, Тесла, Лира, Клевер)
    // [2] Несокрушимый Бастион (Орбиты, Молот, Щиты, Эгида)
    // [3] Аннигилятор Бездны (Луч Смерти, Сингулярность, Око Колосса)
    // [4] Кровавый Жнец (Топор, Кинжалы, Вампиризм, Клевер)
    // [5] Звёздный Охотник (Лук, Ракеты, Самонаведение, Орлиный Глаз)
    #[serde(default = "default_combo_archetype_weights")]
    pub combo_archetype_weights: Vec<f32>,

    // Степень стремления к сбору синергий (1.0 = норма, 2.5 = фанатичный поиск комбо)
    #[serde(default = "default_synergy_seeking_weight")]
    pub synergy_seeking_weight: f32,
}

pub fn default_combo_archetype_weights() -> Vec<f32> {
    vec![1.5, 1.5, 1.5, 1.5, 1.5, 1.5]
}

pub fn default_synergy_seeking_weight() -> f32 {
    1.6
}

pub fn default_weapon_priorities() -> Vec<f32> {
    vec![
        1.2, 1.4, 1.5, 1.3, 1.3, 1.1, 1.4, 1.3, 1.5, 1.4, // 1..10
        1.5, 1.4, 1.6, 1.5, 1.5, 1.6, 1.4, 1.7, 1.6, 1.8, // 11..20
    ]
}

pub fn default_passive_priorities() -> Vec<f32> {
    vec![
        1.3, 1.5, 1.4, 1.4, 1.2, 1.2, 1.5, 1.3, 1.3, 1.4, 1.5, 1.6, 1.4, 1.5, // 1..14 (Базовые пассивки)
        1.8, 1.7, 1.6, 1.8, 2.0, 1.6, 1.5, 1.7, 1.6, 1.6, 1.7, 1.5, 1.6, 1.7, // 15..28 (Легендарные реликвии)
    ]
}

pub fn default_squad_weights() -> Vec<f32> {
    vec![
        1.0, 1.5, 2.5, 1.8, 2.2, 2.0, 2.4, 2.8, // Тиры 1..8 (Орда)
        1.4, 1.8, 2.6, 2.2, 2.5, 2.2, 2.7, 3.2, // Тиры 9..16 (Легион)
    ]
}

impl Default for BossModel {
    fn default() -> Self {
        let net = crate::ai::neural_net::NeuralNet::new();
        let synapses = net.total_synapses();
        Self {
            version: "v2.0-neural".to_string(),
            generation: 0,
            fitness: 0.0,
            avg_survival_time: 0.0,
            total_synapses: synapses,
            neural_net: Some(net),
            mob_repulsion_weight: 440.0,
            gem_attraction_weight: 220.0,
            altar_siege_aggression: 280.0,
            sniper_hunt_aggression: 320.0,
            wall_repel_weight: 560.0,
            dash_danger_threshold: 13,
            retreat_hp_threshold: 0.32,
            sniper_engage_min_count: 2,
            weapon_priorities: default_weapon_priorities(),
            passive_priorities: default_passive_priorities(),
            combo_archetype_weights: default_combo_archetype_weights(),
            synergy_seeking_weight: default_synergy_seeking_weight(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorModel {
    pub version: String,
    pub generation: u32,
    pub fitness: f32,
    pub avg_boss_kill_time: f32,

    // Глубокая многоголовая нейросеть Директора (132 сенсора -> 12,808 синапсов)
    #[serde(default)]
    pub neural_net: Option<crate::ai::neural_net::DirectorBrainNet>,
    #[serde(default)]
    pub total_synapses: usize,

    // Веса предпочтения призыва отрядов (сохранены для обратной совместимости)
    #[serde(default = "default_squad_weights")]
    pub squad_spawn_weights: Vec<f32>,

    // Вес постройки Алтарей Крови (для обратной совместимости)
    pub altar_build_priority: f32,
    pub soul_reserve_threshold: f32,
}

impl Default for DirectorModel {
    fn default() -> Self {
        let net = crate::ai::neural_net::DirectorBrainNet::new();
        let synapses = net.total_synapses();
        Self {
            version: "v2.0-neural-rl".to_string(),
            generation: 0,
            fitness: 0.0,
            avg_boss_kill_time: 0.0,
            neural_net: Some(net),
            total_synapses: synapses,
            squad_spawn_weights: default_squad_weights(),
            altar_build_priority: 0.85,
            soul_reserve_threshold: 40.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainingStats {
    pub total_generations: usize,
    pub total_matches_played: usize,
    pub best_boss_survival_time: f32,
    pub best_boss_level: u32,
    pub best_director_kill_time: f32,
    pub timestamp: String,
}
