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

    // Приоритеты выбора оружия при Level-Up (6 оружий)
    // 0: WindBlade, 1: FireAura, 2: MagicWand, 3: HolyHammer, 4: IceShards, 5: PoisonOrbs
    pub weapon_priorities: [f32; 6],

    // Приоритеты выбора пассивок при Level-Up (6 пассивок)
    // 0: SpeedGloves, 1: GiantHeart, 2: CooldownTome, 3: LightArmor, 4: FrostRing, 5: CorrosiveFlask
    pub passive_priorities: [f32; 6],
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
            weapon_priorities: [1.2, 1.4, 1.5, 1.3, 1.3, 1.1],
            passive_priorities: [1.3, 1.5, 1.4, 1.4, 1.2, 1.2],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorModel {
    pub version: String,
    pub generation: u32,
    pub fitness: f32,
    pub avg_boss_kill_time: f32,

    // Веса предпочтения призыва отрядов (Тиры 1 - 5)
    // 0: Скелеты (15), 1: Гончие (35), 2: Культисты (70), 3: Големы (150), 4: Камикадзе (300)
    pub squad_spawn_weights: [f32; 5],

    // Вес постройки Алтарей Крови
    pub altar_build_priority: f32,       // Шанс построить алтарь сразу при откате (0.0 - 1.0)
    pub soul_reserve_threshold: f32,     // Запас душ перед тратой на дорогие тиры (0.0 - 200.0)
}

impl Default for DirectorModel {
    fn default() -> Self {
        Self {
            version: "v1.0-coev".to_string(),
            generation: 0,
            fitness: 0.0,
            avg_boss_kill_time: 0.0,
            squad_spawn_weights: [1.0, 1.5, 2.5, 1.8, 2.2], // Особый фокус на Культистах и Камикадзе
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
