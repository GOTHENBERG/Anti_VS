use crate::core::rand;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use crate::ai::league::{
    ControlLeagueReport, ControlLeagueRoster, print_control_league_report,
};
use crate::ai::models::{BossModel, DirectorModel, TrainingStats, MODEL_FORMAT_VERSION};
use crate::ai::sensors::TOTAL_SENSOR_SCHEMA_DIM;
use crate::ai::simulation::{run_headless_simulation_with_boss_count, MatchResult};
use crate::ecs::components::inventory::{PassiveType, WeaponType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationManifest {
    pub generation_id: usize,
    pub model_format_version: u32,
    pub sensor_schema_dim: usize,
    pub run_seed: u64,
    pub boss_model_hash: String,
    pub director_model_hash: String,
    pub control_league_digest: Option<String>,
    pub boss_control_fitness: Option<f32>,
    pub boss_control_win_rate: Option<f32>,
    pub director_control_fitness: Option<f32>,
    pub director_control_win_rate: Option<f32>,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedRunSummary {
    pub seed: u64,
    pub generation: u32,
    pub boss_train_fitness: f32,
    pub director_train_fitness: f32,
    pub total_matches: usize,
    pub control_report: ControlLeagueReport,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiSeedMetricStats {
    pub mean: f32,
    pub std_dev: f32,
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiSeedReport {
    pub seeds: Vec<u64>,
    pub generations_per_seed: usize,
    pub population_size: usize,
    pub match_cap_arg: Option<f32>,
    pub baseline_disk: Option<ControlLeagueReport>,
    pub seed_runs: Vec<SeedRunSummary>,
    pub boss_fitness_stats: MultiSeedMetricStats,
    pub boss_win_rate_stats: MultiSeedMetricStats,
    pub boss_survival_secs_stats: MultiSeedMetricStats,
    pub director_fitness_stats: MultiSeedMetricStats,
    pub director_win_rate_stats: MultiSeedMetricStats,
    pub director_kill_time_stats: MultiSeedMetricStats,
    pub best_boss_seed: u64,
    pub best_director_seed: u64,
    pub combined_control_report: ControlLeagueReport,
    pub promoted_to_live_models: bool,
    pub timestamp: String,
}

pub struct SingleSeedOutcome {
    pub seed: u64,
    pub best_boss: BossModel,
    pub best_director: DirectorModel,
    pub total_matches: usize,
    pub best_avg_boss_level: u32,
}

fn fnv1a_64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{:#018x}", hash)
}

pub(crate) fn atomic_write_bytes(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let tmp_name = format!(
        "{}.tmp.{}",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("artifact"),
        std::process::id()
    );
    let tmp_path = path.with_file_name(tmp_name);
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

fn metric_stats(values: &[f32]) -> MultiSeedMetricStats {
    if values.is_empty() {
        return MultiSeedMetricStats { mean: 0.0, std_dev: 0.0, min: 0.0, max: 0.0 };
    }
    let n = values.len() as f32;
    let mean = values.iter().copied().sum::<f32>() / n;
    let var = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
    let min = values.iter().copied().fold(f32::INFINITY, f32::min);
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    MultiSeedMetricStats {
        mean,
        std_dev: var.sqrt(),
        min,
        max,
    }
}

const MATCHES_PER_AGENT: usize = 6;
/// Полная длительность матча (17 минут), к которой стремится curriculum.
const FULL_MATCH_CAP: f32 = 1020.0;
/// Стартовая длительность матча для ускорения ранних поколений.
const START_MATCH_CAP: f32 = 300.0;

fn append_to_history_log(text: &str) {
    let _ = std::fs::create_dir_all("models");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("models/training_history.log")
    {
        use std::io::Write;
        let _ = write!(file, "{}", text);
    }
}

/// Текущее UTC-время в ISO-8601 (без внешних зависимостей).
fn utc_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);

    // Howard Hinnant, civil_from_days
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

fn system_base_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

fn shuffle<T>(items: &mut [T]) {
    if items.len() < 2 {
        return;
    }
    for i in (1..items.len()).rev() {
        let j = rand::gen_range(0usize, i + 1);
        items.swap(i, j);
    }
}

/// Единая полезность Директора. Ноль-суммарный фитнес:
/// Директор максимизирует `director_utility`, Босс максимизирует `-director_utility`.
///
/// Компоненты (максимум ~1400):
/// - 600 за то, как рано закончился матч (доля не прожитого времени);
/// - 400 за убийство Босса;
/// - 200 за долю снятого здоровья Босса;
/// - 150 за среднюю долю времени жизни построенных алтарей;
/// - 50 за конверсию душ в действия (spent / (spent + unspent + 100));
/// - отрицательный бонус Директору за уничтоженные алтари и деактивированные обелиски.
pub(crate) fn director_utility(res: &MatchResult) -> f32 {
    let cap = res.match_cap.max(1.0);
    let survival_frac = (res.survival_time / cap).clamp(0.0, 1.0);
    let time_reward = (1.0 - survival_frac) * 600.0;
    // Дифференцированный бонус за устранение боссов команды (гладкий кредит в коопе)
    let kill_fraction = if res.total_bosses > 0 {
        (res.total_bosses.saturating_sub(res.living_bosses)) as f32 / res.total_bosses as f32
    } else if res.boss_won {
        0.0
    } else {
        1.0
    };
    let kill_bonus = kill_fraction * 400.0;
    let damage_frac = (res.damage_dealt_to_boss / res.boss_max_health.max(1.0)).clamp(0.0, 1.0);
    let damage_reward = damage_frac * 200.0;
    // Награда за удержание алтарей: делится на интервалы по базовому КД 45 секунд.
    // Суммарный бонус за 1020с ограничен 150.0 (как за 17 алтарей при старом кулдауне в 60с).
    let altar_cooldown = crate::content::structures::ALTAR_BASE_COOLDOWN_SECONDS;
    let altar_intervals = (res.survival_time / altar_cooldown).max(1.0);
    let expected_max_uptime = (res.survival_time * (altar_intervals + 1.0) * 0.5).max(altar_cooldown);
    let uptime_frac = (res.total_altar_uptime_seconds / expected_max_uptime).clamp(0.0, 1.0);
    let altar_reward = uptime_frac * 150.0;
    let economy_denom = res.director_points_spent + res.director_points_unspent + 100.0;
    let economy_reward =
        (res.director_points_spent / economy_denom).clamp(0.0, 1.0) * 50.0;
    let boss_structure_reward = res.altars_destroyed.min(12) as f32 * 8.0
        + res.obelisks_deactivated.min(4) as f32 * 25.0;

    (time_reward + kill_bonus + damage_reward + altar_reward + economy_reward
        - boss_structure_reward)
        .clamp(-1400.0, 1400.0)
}

#[derive(Clone, Debug)]
struct BossEval {
    idx: usize,
    fitness: f32,
    avg_survival: f32,
    avg_level: u32,
    avg_damage_taken: f32,
    avg_melee_damage: f32,
    avg_ranged_damage: f32,
    avg_hazard_damage: f32,
    avg_altars_destroyed: usize,
    avg_obelisks_deactivated: usize,
    avg_altars_built: usize,
    avg_meditation_time: f32,
    avg_distance_walked: f32,
    avg_dashes: usize,
    mobs_spawned: [usize; 16],
    director_grades: (u32, u32, u32),
    weapons: Vec<(WeaponType, u8)>,
    passives: Vec<(PassiveType, u8)>,
}

#[derive(Clone, Debug)]
struct DirectorEval {
    idx: usize,
    fitness: f32,
    avg_kill_speed: f32,
    avg_damage_dealt: f32,
    avg_altars_built: usize,
    avg_altars_destroyed: usize,
    avg_obelisks_deactivated: usize,
    avg_altar_uptime_rate: f32,
    avg_altar_uptime_total: f32,
    avg_boss_level: u32,
    avg_boss_meditation: f32,
    avg_boss_distance_walked: f32,
    avg_boss_dashes: usize,
    mobs_spawned: [usize; 16],
    grades: (u32, u32, u32),
    boss_weapons: Vec<(WeaponType, u8)>,
    boss_passives: Vec<(PassiveType, u8)>,
}

#[derive(Clone, Debug, Default)]
struct BossAcc {
    fitness_sum: f32,
    matches: usize,
    survival_sum: f32,
    level_sum: u64,
    damage_taken_sum: f32,
    melee_sum: f32,
    ranged_sum: f32,
    hazard_sum: f32,
    altars_destroyed: usize,
    obelisks_deactivated: usize,
    altars_built: usize,
    meditation_sum: f32,
    distance_sum: f32,
    dashes: usize,
    mobs: [usize; 16],
    grades: (u32, u32, u32),
    weapons: Vec<(WeaponType, u8)>,
    passives: Vec<(PassiveType, u8)>,
    representative_key: f32,
}

impl BossAcc {
    fn add(&mut self, res: &MatchResult) {
        self.fitness_sum += -director_utility(res);
        self.matches += 1;
        self.survival_sum += res.survival_time;
        self.level_sum += res.boss_level as u64;
        self.damage_taken_sum += res.damage_dealt_to_boss;
        self.melee_sum += res.boss_damage_stats.melee_total;
        self.ranged_sum += res.boss_damage_stats.ranged_total;
        self.hazard_sum += res.boss_damage_stats.hazard_total;
        self.altars_destroyed += res.altars_destroyed;
        self.obelisks_deactivated += res.obelisks_deactivated;
        self.altars_built += res.altars_built;
        self.meditation_sum += res.total_meditation_time;
        self.distance_sum += res.distance_walked;
        self.dashes += res.boss_dashes;
        for (dst, src) in self.mobs.iter_mut().zip(res.mobs_spawned_by_tier.iter()) {
            *dst += *src;
        }

        // Для логов берём матч с самым долгим выживанием
        // (первый матч всегда становится представителем, даже если счёт 0)
        if self.matches == 1 || res.survival_time >= self.representative_key {
            self.representative_key = res.survival_time;
            self.grades = res.director_grades;
            self.weapons = res.final_weapons.clone();
            self.passives = res.final_passives.clone();
        }
    }

    fn into_eval(self, idx: usize) -> BossEval {
        let m = self.matches.max(1) as f32;
        let mut mobs = [0usize; 16];
        for t in 0..16 {
            mobs[t] = (self.mobs[t] as f32 / m).round() as usize;
        }
        BossEval {
            idx,
            fitness: self.fitness_sum / m,
            avg_survival: self.survival_sum / m,
            avg_level: (self.level_sum as f32 / m).round() as u32,
            avg_damage_taken: self.damage_taken_sum / m,
            avg_melee_damage: self.melee_sum / m,
            avg_ranged_damage: self.ranged_sum / m,
            avg_hazard_damage: self.hazard_sum / m,
            avg_altars_destroyed: (self.altars_destroyed as f32 / m).round() as usize,
            avg_obelisks_deactivated: (self.obelisks_deactivated as f32 / m).round() as usize,
            avg_altars_built: (self.altars_built as f32 / m).round() as usize,
            avg_meditation_time: self.meditation_sum / m,
            avg_distance_walked: self.distance_sum / m,
            avg_dashes: (self.dashes as f32 / m).round() as usize,
            mobs_spawned: mobs,
            director_grades: self.grades,
            weapons: self.weapons,
            passives: self.passives,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct DirectorAcc {
    fitness_sum: f32,
    matches: usize,
    survival_sum: f32,
    damage_sum: f32,
    altars_built: usize,
    altars_destroyed: usize,
    obelisks_deactivated: usize,
    uptime_sum: f32,
    boss_level_sum: u64,
    boss_meditation_sum: f32,
    boss_distance_sum: f32,
    boss_dashes: usize,
    mobs: [usize; 16],
    grades: (u32, u32, u32),
    boss_weapons: Vec<(WeaponType, u8)>,
    boss_passives: Vec<(PassiveType, u8)>,
    representative_key: f32,
}

impl DirectorAcc {
    fn add(&mut self, res: &MatchResult) {
        let utility = director_utility(res);
        self.fitness_sum += utility;
        self.matches += 1;
        self.survival_sum += res.survival_time;
        self.damage_sum += res.damage_dealt_to_boss;
        self.altars_built += res.altars_built;
        self.altars_destroyed += res.altars_destroyed;
        self.obelisks_deactivated += res.obelisks_deactivated;
        self.uptime_sum += res.total_altar_uptime_seconds;
        self.boss_level_sum += res.boss_level as u64;
        self.boss_meditation_sum += res.total_meditation_time;
        self.boss_distance_sum += res.distance_walked;
        self.boss_dashes += res.boss_dashes;
        for (dst, src) in self.mobs.iter_mut().zip(res.mobs_spawned_by_tier.iter()) {
            *dst += *src;
        }

        // Для логов берём матч с наибольшей полезностью
        // (первый матч всегда становится представителем, даже при отрицательной полезности)
        if self.matches == 1 || utility >= self.representative_key {
            self.representative_key = utility;
            self.grades = res.director_grades;
            self.boss_weapons = res.final_weapons.clone();
            self.boss_passives = res.final_passives.clone();
        }
    }

    fn into_eval(self, idx: usize) -> DirectorEval {
        let m = self.matches.max(1) as f32;
        let avg_kill = self.survival_sum / m;
        let avg_uptime = self.uptime_sum / m;
        let mut mobs = [0usize; 16];
        for t in 0..16 {
            mobs[t] = (self.mobs[t] as f32 / m).round() as usize;
        }
        DirectorEval {
            idx,
            fitness: self.fitness_sum / m,
            avg_kill_speed: avg_kill,
            avg_damage_dealt: self.damage_sum / m,
            avg_altars_built: (self.altars_built as f32 / m).round() as usize,
            avg_altars_destroyed: (self.altars_destroyed as f32 / m).round() as usize,
            avg_obelisks_deactivated: (self.obelisks_deactivated as f32 / m).round() as usize,
            avg_altar_uptime_rate: avg_uptime / (avg_kill / 60.0).max(0.1),
            avg_altar_uptime_total: avg_uptime,
            avg_boss_level: (self.boss_level_sum as f32 / m).round() as u32,
            avg_boss_meditation: self.boss_meditation_sum / m,
            avg_boss_distance_walked: self.boss_distance_sum / m,
            avg_boss_dashes: (self.boss_dashes as f32 / m).round() as usize,
            mobs_spawned: mobs,
            grades: self.grades,
            boss_weapons: self.boss_weapons,
            boss_passives: self.boss_passives,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum AgentRef {
    Population(usize),
    HallOfFame(usize),
}

#[derive(Clone, Copy, Debug)]
struct Pairing {
    boss: AgentRef,
    director: AgentRef,
    seed: u64,
    count_for_boss: bool,
    count_for_director: bool,
}

fn format_build_summary(
    weapons: &[(WeaponType, u8)],
    passives: &[(PassiveType, u8)],
) -> (String, String) {
    let w_str = if weapons.is_empty() {
        "нет".to_string()
    } else {
        weapons.iter()
            .map(|(w, lvl)| format!("{} (ур.{})", crate::content::weapons::get_weapon_def(*w).name, lvl))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let p_str = if passives.is_empty() {
        "нет".to_string()
    } else {
        passives.iter()
            .map(|(p, lvl)| format!("{} (ур.{})", crate::content::passives::get_passive_def(*p).name, lvl))
            .collect::<Vec<_>>()
            .join(", ")
    };
    (w_str, p_str)
}

/// Турнирная селекция: k случайных претендентов, побеждает лучший по фитнесу.
fn tournament_index(fitness: &[f32], k: usize) -> usize {
    let mut best_idx = rand::gen_range(0usize, fitness.len());
    let mut best_fit = fitness[best_idx];
    for _ in 1..k {
        let idx = rand::gen_range(0usize, fitness.len());
        if fitness[idx] > best_fit {
            best_fit = fitness[idx];
            best_idx = idx;
        }
    }
    best_idx
}

#[derive(Clone, Debug)]
pub struct IslandConfig {
    pub island_name: String,
    pub exchange_dir: Option<std::path::PathBuf>,
    pub migrate_every: usize,
}

impl Default for IslandConfig {
    fn default() -> Self {
        Self {
            island_name: "local".to_string(),
            exchange_dir: None,
            migrate_every: 10,
        }
    }
}

pub fn run_co_evolutionary_training(
    generations: usize,
    population_size: usize,
    reset: bool,
    round_duration: Option<f32>,
    boss_decision_interval: f32,
    boss_count: usize,
    island_config: IslandConfig,
    seed_override: Option<u64>,
) {
    let run_seed = seed_override.unwrap_or_else(system_base_seed);
    let outcome = train_co_evolution_seed(
        generations,
        population_size,
        reset,
        round_duration,
        boss_decision_interval,
        boss_count,
        &island_config,
        run_seed,
        true,
    );
    let league_report = ControlLeagueRoster::canonical()
        .evaluate(&outcome.best_boss, &outcome.best_director);
    print_control_league_report(
        &format!("Final Single-Seed Evaluation (seed={})", run_seed),
        &league_report,
    );
    save_models(
        &outcome.best_boss,
        &outcome.best_director,
        outcome.best_boss.generation as usize,
        outcome.total_matches,
        outcome.best_avg_boss_level,
        run_seed,
        Some(&league_report),
        true,
    );
}

pub fn train_co_evolution_seed(
    generations: usize,
    population_size: usize,
    reset: bool,
    round_duration: Option<f32>,
    boss_decision_interval: f32,
    boss_count: usize,
    island_config: &IslandConfig,
    run_seed: u64,
    save_live_checkpoints: bool,
) -> SingleSeedOutcome {
    rand::seed(run_seed);
    let generations = generations.max(1);
    let population_size = population_size.max(3);
    let boss_count = boss_count.clamp(1, 8);
    let duration_info = match round_duration {
        Some(d) => format!("ФИКСИРОВАННЫЙ ({:.0} сек / {:.1} мин)", d, d / 60.0),
        None => "Curriculum (300с -> 1020с)".to_string(),
    };

    println!("============================================================");
    println!(">>> ЗАПУСК ТУРНИРА СО-ЭВОЛЮЦИИ ИИ (LEAGUE SELF-PLAY)");
    println!("  Seed запуска: {} ({:#018x})", run_seed, run_seed);
    println!("  Движок: {} потоков Rayon (ноль-суммарные матчи с фиксированными seed)", rayon::current_num_threads());
    println!("  Поколений: {} | Размер популяции каждого агента: {}", generations, population_size);
    println!("  Длительность матчей: {}", duration_info);
    println!("  Боссов в команде обучения: {}", boss_count);
    println!("  Частота решений Босса: {:.0} мс", boss_decision_interval * 1000.0);
    println!("  Формат: v{} | Сенсоров: {}", MODEL_FORMAT_VERSION, TOTAL_SENSOR_SCHEMA_DIM);
    println!("  Зал Славы (League Hall of Fame): АКТИВЕН (защита от забывания)");
    if let Some(ref dir) = island_config.exchange_dir {
        println!("  [ОСТРОВ] Островная модель: АКТИВНА | Остров: '{}' | Каталог: {:?} | Миграция: каждые {} поколений",
            island_config.island_name, dir, island_config.migrate_every);
    }
    if reset {
        println!("  [СБРОС --clean]: ИИ Босса и Директора очищены, обучение с нуля!");
    }
    println!("============================================================");

    let (seed_boss, seed_director, base_gen) = if reset {
        (BossModel::default(), DirectorModel::default(), 0u32)
    } else {
        let mut boss = crate::ai::load_trained_boss_model().unwrap_or_default();
        let boss_net_ok = boss
            .neural_net
            .as_ref()
            .map_or(false, |n| n.is_compatible());
        if !boss_net_ok {
            boss.neural_net = Some(crate::ai::neural_net::NeuralNet::new());
            boss.generation = 0;
        }
        if let Some(ref net) = boss.neural_net {
            boss.total_synapses = net.total_synapses();
        }
        if boss.fitness_version != MODEL_FORMAT_VERSION || !boss.fitness.is_finite() {
            boss.fitness = f32::NEG_INFINITY;
        }
        boss.model_version = MODEL_FORMAT_VERSION;
        boss.fitness_version = MODEL_FORMAT_VERSION;

        let mut director = crate::ai::load_trained_director_model().unwrap_or_default();
        let director_net_ok = director
            .neural_net
            .as_ref()
            .map_or(false, |n| n.is_compatible());
        if !director_net_ok {
            director.neural_net = Some(crate::ai::neural_net::DirectorBrainNet::new());
            director.generation = 0;
        }
        if let Some(ref net) = director.neural_net {
            director.total_synapses = net.total_synapses();
        }
        if director.fitness_version != MODEL_FORMAT_VERSION || !director.fitness.is_finite() {
            director.fitness = f32::NEG_INFINITY;
        }
        director.model_version = MODEL_FORMAT_VERSION;
        director.fitness_version = MODEL_FORMAT_VERSION;

        let base = if boss_net_ok && director_net_ok { boss.generation } else { 0 };
        (boss, director, base)
    };

    if base_gen > 0 {
        println!(
            "[ПРОДОЛЖЕНИЕ] Дообучение с поколения {} (валидный фитнес: {:.1})",
            base_gen, seed_boss.fitness
        );
    } else {
        println!(
            "[СТАРТ] Новый цикл: Босс {} синапсов, Директор {} синапсов.",
            seed_boss.total_synapses, seed_director.total_synapses
        );
    }

    let start_msg = format!(
        "\n================================================================================\n\
         >>> СЕССИЯ ОБУЧЕНИЯ (Старт с поколения #{}, запланировано раундов: {}, популяция: {})\n\
         ================================================================================\n",
        base_gen, generations, population_size
    );
    append_to_history_log(&start_msg);

    // 1. Инициализация Зала Славы историческими чемпионами
    let mut hof_directors: Vec<DirectorModel> = vec![seed_director.clone()];
    let mut hof_bosses: Vec<BossModel> = vec![seed_boss.clone()];

    // 2. Стартовые популяции
    let mut boss_pop: Vec<BossModel> = (0..population_size)
        .map(|i| {
            let mut m = seed_boss.clone();
            if i > 0 {
                if let Some(ref mut net) = m.neural_net {
                    net.mutate(0.12, 0.20);
                }
                for a in m.combo_archetype_weights.iter_mut() {
                    *a = (*a + rand::gen_range(-0.4, 0.4)).clamp(0.2, 4.0);
                }
                for w in m.weapon_priorities.iter_mut() {
                    *w = (*w + rand::gen_range(-0.3, 0.3)).clamp(0.2, 3.5);
                }
                for p in m.passive_priorities.iter_mut() {
                    *p = (*p + rand::gen_range(-0.3, 0.3)).clamp(0.2, 3.5);
                }
            }
            m
        })
        .collect();

    let mut director_pop: Vec<DirectorModel> = (0..population_size)
        .map(|i| {
            let mut d = seed_director.clone();
            if i > 0 {
                if let Some(ref mut net) = d.neural_net {
                    net.mutate(0.12, 0.20);
                }
            }
            d
        })
        .collect();

    // Защита от Stale Champion: в текущей серии обучения чемпион выбирается
    // исключительно по реальным результатам текущего прогона (старт с -inf).
    let mut best_overall_boss = seed_boss.clone();
    let mut best_overall_boss_fitness = f32::NEG_INFINITY;
    let mut best_overall_director = seed_director.clone();
    let mut best_overall_dir_fitness = f32::NEG_INFINITY;
    let mut best_avg_boss_level = 0u32;
    let mut total_matches_played = 0usize;
    let mut boss_sigma = 0.20f32;
    let mut director_sigma = 0.18f32;
    let mut prev_best_boss_fitness = f32::NEG_INFINITY;
    let mut prev_best_director_fitness = f32::NEG_INFINITY;
    let base_seed = run_seed;

    // 3. Цикл поколений обучения
    for gen_idx in 1..=generations {
        let current_gen_num = base_gen + gen_idx as u32;

        // Выбор длительности матча: фиксированная (300с / 1020с) либо Curriculum
        let match_cap = if let Some(fixed_duration) = round_duration {
            fixed_duration.max(30.0)
        } else {
            let progress = if generations > 1 {
                (gen_idx - 1) as f32 / (generations - 1) as f32
            } else {
                1.0
            };
            let curriculum_start = if base_gen > 0 { FULL_MATCH_CAP } else { START_MATCH_CAP };
            ((curriculum_start + (FULL_MATCH_CAP - curriculum_start) * progress) / 30.0).round() * 30.0
        };

        // 3.1 Формируем общую матрицу матчей: каждый агент играет ровно
        // MATCHES_PER_AGENT матчей в лиге + один матч против чемпиона Зала Славы.
        let mut pairings: Vec<Pairing> = Vec::with_capacity(population_size * (MATCHES_PER_AGENT + 2));
        let mut boss_order: Vec<usize> = (0..population_size).collect();
        let mut director_order: Vec<usize> = (0..population_size).collect();

        let common_round_seeds: Vec<u64> = (0..MATCHES_PER_AGENT)
            .map(|r| {
                base_seed
                    .wrapping_add((gen_idx as u64) << 40)
                    .wrapping_add(((r + 1) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
            })
            .collect();

        for round in 0..MATCHES_PER_AGENT {
            shuffle(&mut boss_order);
            shuffle(&mut director_order);
            let round_seed = common_round_seeds[round];
            for i in 0..population_size {
                pairings.push(Pairing {
                    boss: AgentRef::Population(boss_order[i]),
                    director: AgentRef::Population(director_order[i]),
                    seed: round_seed,
                    count_for_boss: true,
                    count_for_director: true,
                });
            }
        }

        // Boss vs Hall of Fame (считается только Боссу)
        for b in 0..population_size {
            let h = rand::gen_range(0usize, hof_directors.len());
            let seed = base_seed
                .wrapping_add((gen_idx as u64) << 44)
                .wrapping_add((b as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
            pairings.push(Pairing {
                boss: AgentRef::Population(b),
                director: AgentRef::HallOfFame(h),
                seed,
                count_for_boss: true,
                count_for_director: false,
            });
        }

        // Director vs Hall of Fame (считается только Директору)
        for d in 0..population_size {
            let h = rand::gen_range(0usize, hof_bosses.len());
            let seed = base_seed
                .wrapping_add((gen_idx as u64) << 48)
                .wrapping_add((d as u64).wrapping_mul(0xA24B_AED4_963E_E407));
            pairings.push(Pairing {
                boss: AgentRef::HallOfFame(h),
                director: AgentRef::Population(d),
                seed,
                count_for_boss: false,
                count_for_director: true,
            });
        }

        // 3.2 Параллельно проигрываем всю матрицу. Сид фиксирован на матч,
        // поэтому результаты воспроизводимы и не зависят от порядка потоков.
        let results: Vec<MatchResult> = pairings
            .par_iter()
            .map(|p| {
                let boss = match p.boss {
                    AgentRef::Population(i) => &boss_pop[i],
                    AgentRef::HallOfFame(i) => &hof_bosses[i],
                };
                let director = match p.director {
                    AgentRef::Population(i) => &director_pop[i],
                    AgentRef::HallOfFame(i) => &hof_directors[i],
                };
                run_headless_simulation_with_boss_count(
                    boss,
                    director,
                    p.seed,
                    match_cap,
                    boss_decision_interval,
                    boss_count,
                )
            })
            .collect();

        total_matches_played += results.len();

        // 3.3 Собираем ноль-суммарный фитнес
        let mut boss_acc: Vec<BossAcc> = vec![BossAcc::default(); population_size];
        let mut director_acc: Vec<DirectorAcc> = vec![DirectorAcc::default(); population_size];

        for (pairing, res) in pairings.iter().zip(results.iter()) {
            match pairing.boss {
                AgentRef::Population(idx) if pairing.count_for_boss => {
                    boss_acc[idx].add(res);
                }
                _ => {}
            }
            match pairing.director {
                AgentRef::Population(idx) if pairing.count_for_director => {
                    director_acc[idx].add(res);
                }
                _ => {}
            }
        }

        let boss_results: Vec<BossEval> = boss_acc
            .into_iter()
            .enumerate()
            .map(|(idx, acc)| acc.into_eval(idx))
            .collect();
        let director_results: Vec<DirectorEval> = director_acc
            .into_iter()
            .enumerate()
            .map(|(idx, acc)| acc.into_eval(idx))
            .collect();

        for eval in &boss_results {
            boss_pop[eval.idx].fitness = eval.fitness;
            boss_pop[eval.idx].avg_survival_time = eval.avg_survival;
            boss_pop[eval.idx].generation = current_gen_num;
            boss_pop[eval.idx].model_version = MODEL_FORMAT_VERSION;
            boss_pop[eval.idx].fitness_version = MODEL_FORMAT_VERSION;
        }
        for eval in &director_results {
            director_pop[eval.idx].fitness = eval.fitness;
            director_pop[eval.idx].avg_boss_kill_time = eval.avg_kill_speed;
            director_pop[eval.idx].generation = current_gen_num;
            director_pop[eval.idx].model_version = MODEL_FORMAT_VERSION;
            director_pop[eval.idx].fitness_version = MODEL_FORMAT_VERSION;
        }

        boss_pop.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));
        director_pop.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        let current_best_boss = boss_pop[0].clone();
        let current_best_dir = director_pop[0].clone();

        if current_best_boss.fitness > best_overall_boss_fitness {
            best_overall_boss_fitness = current_best_boss.fitness;
            best_overall_boss = current_best_boss.clone();
        }
        if current_best_dir.fitness > best_overall_dir_fitness {
            best_overall_dir_fitness = current_best_dir.fitness;
            best_overall_director = current_best_dir.clone();
        }

        let best_boss_eval = boss_results
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let best_dir_eval = director_results
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        best_avg_boss_level = best_avg_boss_level.max(best_boss_eval.avg_level);

        // Добавляем текущих лидеров в Зал Славы Лиги
        if hof_bosses.len() < 12 {
            hof_bosses.push(current_best_boss.clone());
        } else {
            let replace_idx = rand::gen_range(1, hof_bosses.len());
            hof_bosses[replace_idx] = current_best_boss.clone();
        }
        if hof_directors.len() < 12 {
            hof_directors.push(current_best_dir.clone());
        } else {
            let replace_idx = rand::gen_range(1, hof_directors.len());
            hof_directors[replace_idx] = current_best_dir.clone();
        }

        // Адаптивная сила мутации: застой -> шире поиск, прогресс -> тоньше настройка
        if prev_best_boss_fitness.is_finite() {
            if best_boss_eval.fitness <= prev_best_boss_fitness + 0.5 {
                boss_sigma = (boss_sigma * 1.08).min(0.60);
            } else {
                boss_sigma = (boss_sigma * 0.92).max(0.06);
            }
            prev_best_boss_fitness = prev_best_boss_fitness.max(best_boss_eval.fitness);
        } else {
            prev_best_boss_fitness = best_boss_eval.fitness;
        }

        if prev_best_director_fitness.is_finite() {
            if best_dir_eval.fitness <= prev_best_director_fitness + 0.5 {
                director_sigma = (director_sigma * 1.08).min(0.60);
            } else {
                director_sigma = (director_sigma * 0.92).max(0.06);
            }
            prev_best_director_fitness = prev_best_director_fitness.max(best_dir_eval.fitness);
        } else {
            prev_best_director_fitness = best_dir_eval.fitness;
        }

        let total_mobs_spawned: usize = best_dir_eval.mobs_spawned.iter().sum();
        let boss_mobs_spawned: usize = best_boss_eval.mobs_spawned.iter().sum();

        let (boss_w_str, boss_p_str) = format_build_summary(&best_boss_eval.weapons, &best_boss_eval.passives);
        let (dir_boss_w_str, dir_boss_p_str) = format_build_summary(&best_dir_eval.boss_weapons, &best_dir_eval.boss_passives);

        let mut round_log = String::with_capacity(2048);
        macro_rules! log_line {
            ($($arg:tt)*) => {{
                let s = format!($($arg)*);
                println!("{}", s);
                round_log.push_str(&s);
                round_log.push('\n');
            }};
        }

        log_line!("================================================================================");
        log_line!(
            ">>> РАУНД {:02}/{:02} (Поколение #{:02}) [Зал Славы: {} Боссов / {} Директоров]",
            gen_idx, generations, current_gen_num, hof_bosses.len(), hof_directors.len()
        );
        log_line!(
            "    Матчей: {} (по {} + 1 против Зала Славы на агента) | Лимит матча: {:.0} сек | Sigma: Boss {:.3} / Dir {:.3}",
            results.len(), MATCHES_PER_AGENT, match_cap, boss_sigma, director_sigma
        );
        log_line!("--------------------------------------------------------------------------------");
        log_line!("  [ВРЕМЯ ЖИЗНИ БОССА В РАУНДЕ]:");
        log_line!(
            "    >> В матчах лучшего Босса:     {:>4.1} мин ({:>5.1} сек)  [лимит: {:.1} мин]",
            best_boss_eval.avg_survival / 60.0,
            best_boss_eval.avg_survival,
            match_cap / 60.0
        );
        log_line!(
            "    >> Против лучшего Директора:   {:>4.1} мин ({:>5.1} сек)",
            best_dir_eval.avg_kill_speed / 60.0,
            best_dir_eval.avg_kill_speed
        );
        log_line!("--------------------------------------------------------------------------------");
        log_line!(
            "  БОСС (Фитнес: {:+.1}, Уровень: {}):",
            best_boss_eval.fitness, best_boss_eval.avg_level
        );
        log_line!(
            "    * Время жизни: {:>4.1} мин ({:>5.1} сек) | Медитация: {:.1} мин | Рывков: {}",
            best_boss_eval.avg_survival / 60.0,
            best_boss_eval.avg_survival,
            best_boss_eval.avg_meditation_time / 60.0,
            best_boss_eval.avg_dashes
        );
        let boss_speed = if best_boss_eval.avg_survival > 0.0 {
            best_boss_eval.avg_distance_walked / best_boss_eval.avg_survival
        } else {
            0.0
        };
        log_line!(
            "    * Пройдено пешком: {:.0} px | Ср. скорость: {:.1} px/сек",
            best_boss_eval.avg_distance_walked,
            boss_speed
        );
        log_line!(
            "    * Фитнес = -Полезность Директора (ноль-сумма). Урон получен: {:.0} HP [Ближний: {:.0}, Дальний: {:.0}, Окружение: {:.0}]",
            best_boss_eval.avg_damage_taken,
            best_boss_eval.avg_melee_damage,
            best_boss_eval.avg_ranged_damage,
            best_boss_eval.avg_hazard_damage
        );
        log_line!(
            "    * Разрушено алтарей: {} | Деактивировано обелисков: {} | Скиллы ({}): [{}]",
            best_boss_eval.avg_altars_destroyed,
            best_boss_eval.avg_obelisks_deactivated,
            best_boss_eval.weapons.len(),
            boss_w_str
        );
        log_line!(
            "    * Пассивки ({}): [{}]",
            best_boss_eval.passives.len(),
            boss_p_str
        );
        log_line!(
            "    ↳ Противник: алтарей построено {} | Призвано мобов: {} | Грейды: [Ярость {}, Панцирь {}, Споры {}]",
            best_boss_eval.avg_altars_built,
            boss_mobs_spawned,
            best_boss_eval.director_grades.0,
            best_boss_eval.director_grades.1,
            best_boss_eval.director_grades.2
        );
        log_line!(
            "  ДИРЕКТОР (Фитнес: {:+.1}):",
            best_dir_eval.fitness
        );
        log_line!(
            "    * Время жизни Босса: {:>4.1} мин ({:>5.1} сек) | Урон: {:.0} HP | Алтари: {} (uptime {:.1} с/мин)",
            best_dir_eval.avg_kill_speed / 60.0,
            best_dir_eval.avg_kill_speed,
            best_dir_eval.avg_damage_dealt,
            best_dir_eval.avg_altars_built,
            best_dir_eval.avg_altar_uptime_rate
        );
        log_line!(
            "    * Построено алтарей: {} [Сумм. жизнь: {:.1} мин] | Призвано мобов: {} | Грейды: [Ярость {}, Панцирь {}, Споры {}]",
            best_dir_eval.avg_altars_built,
            best_dir_eval.avg_altar_uptime_total / 60.0,
            total_mobs_spawned,
            best_dir_eval.grades.0, best_dir_eval.grades.1, best_dir_eval.grades.2
        );
        log_line!(
            "      - Орда (T1-8): [Скелеты: {}, Гончие: {}, Культисты: {}, Големы: {}, Камикадзе: {}, Демоны: {}, Огонь: {}, Жнецы: {}]",
            best_dir_eval.mobs_spawned[0], best_dir_eval.mobs_spawned[1], best_dir_eval.mobs_spawned[2], best_dir_eval.mobs_spawned[3],
            best_dir_eval.mobs_spawned[4], best_dir_eval.mobs_spawned[5], best_dir_eval.mobs_spawned[6], best_dir_eval.mobs_spawned[7],
        );
        let legion_total: usize = best_dir_eval.mobs_spawned[8..16].iter().sum();
        if legion_total > 0 {
            log_line!(
                "      - Легион (T9-16): [Фаланга: {}, Церберы: {}, Архикультисты: {}, Колоссы: {}, Мучители: {}, Ловчие: {}, Архонты: {}, Аватар: {}]",
                best_dir_eval.mobs_spawned[8], best_dir_eval.mobs_spawned[9], best_dir_eval.mobs_spawned[10], best_dir_eval.mobs_spawned[11],
                best_dir_eval.mobs_spawned[12], best_dir_eval.mobs_spawned[13], best_dir_eval.mobs_spawned[14], best_dir_eval.mobs_spawned[15],
            );
        }
        log_line!(
            "    ↳ Противник (Босс): Уровень {} | Пешком {:.0} px [Рывков {}] | Медитация {:.1} мин | Разрушил алтарей {} / обелисков {}",
            best_dir_eval.avg_boss_level,
            best_dir_eval.avg_boss_distance_walked,
            best_dir_eval.avg_boss_dashes,
            best_dir_eval.avg_boss_meditation / 60.0,
            best_dir_eval.avg_altars_destroyed,
            best_dir_eval.avg_obelisks_deactivated
        );
        log_line!(
            "    ↳ Скиллы Босса ({}): [{}]",
            best_dir_eval.boss_weapons.len(),
            dir_boss_w_str
        );
        log_line!(
            "    ↳ Пассивки Босса ({}): [{}]",
            best_dir_eval.boss_passives.len(),
            dir_boss_p_str
        );
        log_line!("================================================================================");

        append_to_history_log(&round_log);

        // 3.4 Островная модель: экспорт чемпионов и импорт иммигрантов между устройствами
        if island_config.exchange_dir.is_some() && gen_idx % island_config.migrate_every == 0 {
            process_island_migration(
                &island_config,
                current_gen_num,
                &current_best_boss,
                &current_best_dir,
                &mut boss_pop,
                &mut director_pop,
                &mut hof_bosses,
                &mut hof_directors,
            );
        }

        // 4. Селекция: элита + турнир; нейросеть клонируется и мутирует,
        //    скалярные генетические приоритеты смешиваются и мутируют.
        let mut next_boss_pop = vec![boss_pop[0].clone(), boss_pop[1].clone()];
        let boss_fitness: Vec<f32> = boss_pop.iter().map(|m| m.fitness).collect();

        while next_boss_pop.len() < population_size {
            let p1_idx = tournament_index(&boss_fitness, 3);
            let p2_idx = tournament_index(&boss_fitness, 3);
            let p1 = &boss_pop[p1_idx];
            let p2 = &boss_pop[p2_idx];
            let mut child = p1.clone();

            if let Some(net1) = &p1.neural_net {
                // Сохранение скрытых нейронов одного родителя избегает
                // покомпонентного скрещивания несогласованных нейронных признаков.
                let mut child_net = net1.clone();
                child_net.mutate(0.08, boss_sigma);
                child_net.decay(0.002);
                child.total_synapses = child_net.total_synapses();
                child.neural_net = Some(child_net);
            } else {
                let mut net = crate::ai::neural_net::NeuralNet::new();
                net.mutate(0.10, boss_sigma);
                child.total_synapses = net.total_synapses();
                child.neural_net = Some(net);
            }

            for (w, p2_w) in child.weapon_priorities.iter_mut().zip(p2.weapon_priorities.iter()) {
                if rand::gen_range(0.0, 1.0) < 0.25 {
                    *w = ((*w + *p2_w) * 0.5 + rand::gen_range(-0.25, 0.25)).clamp(0.2, 3.5);
                }
            }
            for (p, p2_p) in child.passive_priorities.iter_mut().zip(p2.passive_priorities.iter()) {
                if rand::gen_range(0.0, 1.0) < 0.25 {
                    *p = ((*p + *p2_p) * 0.5 + rand::gen_range(-0.25, 0.25)).clamp(0.2, 3.5);
                }
            }
            for (a, p2_a) in child.combo_archetype_weights.iter_mut().zip(p2.combo_archetype_weights.iter()) {
                if rand::gen_range(0.0, 1.0) < 0.30 {
                    *a = ((*a + *p2_a) * 0.5 + rand::gen_range(-0.30, 0.30)).clamp(0.2, 4.0);
                }
            }
            child.synergy_seeking_weight = (((p1.synergy_seeking_weight + p2.synergy_seeking_weight) * 0.5) + rand::gen_range(-0.15, 0.15)).clamp(0.5, 3.5);

            next_boss_pop.push(child);
        }

        let mut next_dir_pop = vec![director_pop[0].clone(), director_pop[1].clone()];
        let director_fitness: Vec<f32> = director_pop.iter().map(|m| m.fitness).collect();

        while next_dir_pop.len() < population_size {
            let p1_idx = tournament_index(&director_fitness, 3);
            let p1 = &director_pop[p1_idx];
            let mut child = p1.clone();

            if let Some(net1) = &p1.neural_net {
                // Сохраняем выученную структуру скрытых нейронов родителя p1.
                let mut child_net = net1.clone();
                child_net.mutate(0.08, director_sigma);
                child_net.decay(0.002);
                child.total_synapses = child_net.total_synapses();
                child.neural_net = Some(child_net);
            } else {
                let mut net = crate::ai::neural_net::DirectorBrainNet::new();
                net.mutate(0.10, director_sigma);
                child.total_synapses = net.total_synapses();
                child.neural_net = Some(net);
            }

            next_dir_pop.push(child);
        }

        boss_pop = next_boss_pop;
        director_pop = next_dir_pop;

        // Авто-сохранение каждые 25 поколений (если разрешена живая запись)
        if save_live_checkpoints && gen_idx % 25 == 0 && gen_idx < generations {
            save_models(
                &best_overall_boss,
                &best_overall_director,
                base_gen as usize + gen_idx,
                total_matches_played,
                best_avg_boss_level,
                run_seed,
                None,
                false,
            );
        }
    }

    SingleSeedOutcome {
        seed: run_seed,
        best_boss: best_overall_boss,
        best_director: best_overall_director,
        total_matches: total_matches_played,
        best_avg_boss_level,
    }
}

/// Запускает обучение на нескольких независимых seed с единой оценкой на Контрольной Лиге,
/// публикует разброс качества (mean ± std) и атомарно обновляет модели при победе в Лиге (или `--force-promote`).
#[allow(clippy::too_many_arguments)]
pub fn run_multi_seed_training_and_league(
    seeds: &[u64],
    generations: usize,
    population_size: usize,
    reset: bool,
    round_duration: Option<f32>,
    boss_decision_interval: f32,
    boss_count: usize,
    island_config: &IslandConfig,
    force_promote: bool,
) -> MultiSeedReport {
    let league = ControlLeagueRoster::canonical();
    let disk_boss = crate::ai::load_trained_boss_model();
    let disk_dir = crate::ai::load_trained_director_model();
    let baseline_disk = match (&disk_boss, &disk_dir) {
        (Some(b), Some(d)) => {
            let rep = league.evaluate(b, d);
            print_control_league_report("Baseline On-Disk Models (Pre-Run)", &rep);
            Some(rep)
        }
        _ => None,
    };

    let default_rep = {
        rand::seed(0xDEF0_0001);
        let b = BossModel::default();
        let d = DirectorModel::default();
        league.evaluate(&b, &d)
    };
    print_control_league_report("Untrained Default Baseline", &default_rep);

    let mut seed_runs = Vec::with_capacity(seeds.len());
    let mut seed_bosses: Vec<(u64, BossModel, SingleSeedOutcome)> = Vec::with_capacity(seeds.len());
    let mut seed_dirs: Vec<(u64, DirectorModel)> = Vec::with_capacity(seeds.len());

    for &seed in seeds {
        let outcome = train_co_evolution_seed(
            generations,
            population_size,
            reset,
            round_duration,
            boss_decision_interval,
            boss_count,
            island_config,
            seed,
            false,
        );
        let rep = league.evaluate(&outcome.best_boss, &outcome.best_director);
        print_control_league_report(&format!("Seed {} Champion", seed), &rep);

        let seed_dir = format!("models/seeds/seed_{}", seed);
        if let Ok(b_json) = serde_json::to_vec_pretty(&outcome.best_boss) {
            let _ = atomic_write_bytes(std::path::Path::new(&format!("{}/boss_policy.json", seed_dir)), &b_json);
        }
        if let Ok(d_json) = serde_json::to_vec_pretty(&outcome.best_director) {
            let _ = atomic_write_bytes(std::path::Path::new(&format!("{}/director_policy.json", seed_dir)), &d_json);
        }
        if let Ok(r_json) = serde_json::to_vec_pretty(&rep) {
            let _ = atomic_write_bytes(std::path::Path::new(&format!("{}/control_report.json", seed_dir)), &r_json);
        }

        seed_runs.push(SeedRunSummary {
            seed,
            generation: outcome.best_boss.generation,
            boss_train_fitness: outcome.best_boss.fitness,
            director_train_fitness: outcome.best_director.fitness,
            total_matches: outcome.total_matches,
            control_report: rep,
        });
        let best_b = outcome.best_boss.clone();
        let best_d = outcome.best_director.clone();
        seed_bosses.push((seed, best_b, outcome));
        seed_dirs.push((seed, best_d));
    }

    let boss_fits: Vec<f32> = seed_runs.iter().map(|s| s.control_report.boss.fitness_mean).collect();
    let boss_wins: Vec<f32> = seed_runs.iter().map(|s| s.control_report.boss.win_rate * 100.0).collect();
    let boss_survs: Vec<f32> = seed_runs.iter().map(|s| s.control_report.boss.avg_survival_secs).collect();
    let dir_fits: Vec<f32> = seed_runs.iter().map(|s| s.control_report.director.fitness_mean).collect();
    let dir_wins: Vec<f32> = seed_runs.iter().map(|s| s.control_report.director.win_rate * 100.0).collect();
    let dir_kills: Vec<f32> = seed_runs.iter().map(|s| s.control_report.director.avg_kill_time_secs).collect();

    let boss_fitness_stats = metric_stats(&boss_fits);
    let boss_win_rate_stats = metric_stats(&boss_wins);
    let boss_survival_secs_stats = metric_stats(&boss_survs);
    let director_fitness_stats = metric_stats(&dir_fits);
    let director_win_rate_stats = metric_stats(&dir_wins);
    let director_kill_time_stats = metric_stats(&dir_kills);

    let best_boss_idx = seed_runs
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.control_report.boss.fitness_mean.total_cmp(&b.1.control_report.boss.fitness_mean))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let best_dir_idx = seed_runs
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.control_report.director.fitness_mean.total_cmp(&b.1.control_report.director.fitness_mean))
        .map(|(i, _)| i)
        .unwrap_or(0);

    let best_boss_seed = seed_runs[best_boss_idx].seed;
    let best_director_seed = seed_runs[best_dir_idx].seed;
    let mut final_boss = seed_bosses[best_boss_idx].1.clone();
    let mut final_dir = seed_dirs[best_dir_idx].1.clone();
    let total_matches_all: usize = seed_bosses.iter().map(|(_, _, o)| o.total_matches).sum();
    let max_boss_level = seed_bosses.iter().map(|(_, _, o)| o.best_avg_boss_level).max().unwrap_or(1);

    // По каждой роли отдельно проверяем превосходство над прошлой моделью на диске в Контрольной Лиге.
    let mut boss_promoted = force_promote || baseline_disk.is_none();
    if let Some(ref base) = baseline_disk {
        if seed_runs[best_boss_idx].control_report.boss.fitness_mean >= base.boss.fitness_mean {
            boss_promoted = true;
        } else if !force_promote {
            if let Some(existing) = disk_boss {
                final_boss = existing;
            }
        }
    }
    let mut dir_promoted = force_promote || baseline_disk.is_none();
    if let Some(ref base) = baseline_disk {
        if seed_runs[best_dir_idx].control_report.director.fitness_mean >= base.director.fitness_mean {
            dir_promoted = true;
        } else if !force_promote {
            if let Some(existing) = disk_dir {
                final_dir = existing;
            }
        }
    }

    let combined_control_report = league.evaluate(&final_boss, &final_dir);
    println!("\n==========================================================================");
    println!(" [MULTI-SEED SUMMARY across {} seeds: {:?}]", seeds.len(), seeds);
    println!(
        "   Boss Control Fit: {:+7.2} ± {:5.2} (min {:+7.2}, max {:+7.2}) | Win%: {:4.1} ± {:4.1}% | Surv: {:.1} ± {:.1}s",
        boss_fitness_stats.mean, boss_fitness_stats.std_dev, boss_fitness_stats.min, boss_fitness_stats.max,
        boss_win_rate_stats.mean, boss_win_rate_stats.std_dev,
        boss_survival_secs_stats.mean, boss_survival_secs_stats.std_dev
    );
    println!(
        "   Dir  Control Fit: {:+7.2} ± {:5.2} (min {:+7.2}, max {:+7.2}) | Win%: {:4.1} ± {:4.1}% | End : {:.1} ± {:.1}s",
        director_fitness_stats.mean, director_fitness_stats.std_dev, director_fitness_stats.min, director_fitness_stats.max,
        director_win_rate_stats.mean, director_win_rate_stats.std_dev,
        director_kill_time_stats.mean, director_kill_time_stats.std_dev
    );
    println!(
        "   Selected: Boss from seed {} (promoted={}) | Director from seed {} (promoted={})",
        best_boss_seed, boss_promoted, best_director_seed, dir_promoted
    );
    print_control_league_report("FINAL SELECTED MODELS", &combined_control_report);

    let promoted = boss_promoted || dir_promoted;
    if promoted {
        save_models(
            &final_boss,
            &final_dir,
            final_boss.generation.max(final_dir.generation) as usize,
            total_matches_all,
            max_boss_level,
            best_boss_seed,
            Some(&combined_control_report),
            true,
        );
    } else {
        println!("[ЛИГА] Прежние модели на диске показали более высокий результат — боевые файлы не перезаписаны.");
    }

    let report = MultiSeedReport {
        seeds: seeds.to_vec(),
        generations_per_seed: generations,
        population_size,
        match_cap_arg: round_duration,
        baseline_disk,
        seed_runs,
        boss_fitness_stats,
        boss_win_rate_stats,
        boss_survival_secs_stats,
        director_fitness_stats,
        director_win_rate_stats,
        director_kill_time_stats,
        best_boss_seed,
        best_director_seed,
        combined_control_report,
        promoted_to_live_models: promoted,
        timestamp: utc_timestamp(),
    };
    if let Ok(json) = serde_json::to_vec_pretty(&report) {
        let _ = atomic_write_bytes(std::path::Path::new("models/control_league_report.json"), &json);
    }
    report
}

/// Запускает оценку текущих моделей на диске (и базовых эталонов) на Контрольной Лиге.
pub fn run_control_league_cli() {
    let league = ControlLeagueRoster::canonical();
    rand::seed(0xDEF0_0001);
    let default_boss = BossModel::default();
    let default_dir = DirectorModel::default();
    let default_rep = league.evaluate(&default_boss, &default_dir);
    print_control_league_report("Default Untrained Models", &default_rep);

    let disk_boss = crate::ai::load_trained_boss_model();
    let disk_dir = crate::ai::load_trained_director_model();
    if let (Some(b), Some(d)) = (disk_boss, disk_dir) {
        let rep = league.evaluate(&b, &d);
        print_control_league_report("Active On-Disk Models (models/*_policy.json)", &rep);
    } else {
        println!("[ЛИГА] Обученные модели в models/*_policy.json не найдены.");
    }
}

fn save_models(
    boss: &BossModel,
    director: &DirectorModel,
    generations_total: usize,
    matches_played: usize,
    best_boss_level: u32,
    run_seed: u64,
    league_report: Option<&ControlLeagueReport>,
    is_final: bool,
) {
    let _ = std::fs::create_dir_all("models");
    if is_final {
        println!("\n[СОХРАНЕНИЕ] Финальный атомарный экспорт обученных моделей...");
    } else {
        println!("\n[ЧЕКПОИНТ #{}] Атомарное сохранение прогресса на диск...", boss.generation);
    }

    let mut boss_out = boss.clone();
    boss_out.model_version = MODEL_FORMAT_VERSION;
    boss_out.fitness_version = MODEL_FORMAT_VERSION;
    boss_out.version = format!("v{}.0-coevolution", MODEL_FORMAT_VERSION);
    if let Some(ref net) = boss_out.neural_net {
        boss_out.total_synapses = net.total_synapses();
    }

    let mut director_out = director.clone();
    director_out.model_version = MODEL_FORMAT_VERSION;
    director_out.fitness_version = MODEL_FORMAT_VERSION;
    director_out.version = format!("v{}.0-coevolution", MODEL_FORMAT_VERSION);
    if let Some(ref net) = director_out.neural_net {
        director_out.total_synapses = net.total_synapses();
    }

    let mut boss_hash = String::new();
    if let Ok(boss_json) = serde_json::to_vec_pretty(&boss_out) {
        boss_hash = fnv1a_64_hex(&boss_json);
        if atomic_write_bytes(std::path::Path::new("models/boss_policy.json"), &boss_json).is_ok() {
            println!(
                "  [OK] Модель Босса ({} синапсов, Поколение #{}) -> models/boss_policy.json",
                boss_out.total_synapses, boss_out.generation
            );
        }
    }

    let archetype_names = [
        "Инфернальный Пиромант (Огонь, Пламя, Сердце Пироманта)",
        "Сверхпроводящий Громовержец (Молния, Тесла, Лира, Монета Удачи)",
        "Несокрушимый Бастион (Орбиты, Молот, Щиты, Филактерия)",
        "Аннигилятор Бездны (Луч Смерти, Сингулярность, Око Колосса)",
        "Кровавый Жнец (Топор, Кинжалы, Вампиризм, Монета Удачи)",
        "Звёздный Охотник (Лук, Ракеты, Самонаведение, Орлиный Глаз)",
    ];
    let mut best_arch = 0;
    let mut best_weight = 0.0f32;
    for (i, &w) in boss_out.combo_archetype_weights.iter().enumerate() {
        if w > best_weight {
            best_weight = w;
            best_arch = i;
        }
    }
    if let Some(name) = archetype_names.get(best_arch) {
        println!(
            "  [АРХЕТИП] Доминирующий билд: {} (Вес: {:.2}, Поиск комбо: {:.2})",
            name, best_weight, boss_out.synergy_seeking_weight
        );
    }

    let mut dir_hash = String::new();
    if let Ok(dir_json) = serde_json::to_vec_pretty(&director_out) {
        dir_hash = fnv1a_64_hex(&dir_json);
        if atomic_write_bytes(std::path::Path::new("models/director_policy.json"), &dir_json).is_ok() {
            println!(
                "  [OK] Модель Директора ({} синапсов, Поколение #{}) -> models/director_policy.json",
                director_out.total_synapses, director_out.generation
            );
        }
    }

    let ts = utc_timestamp();
    let stats = TrainingStats {
        total_generations: generations_total,
        total_matches_played: matches_played,
        best_boss_survival_time: boss_out.avg_survival_time,
        best_boss_level,
        best_director_kill_time: director_out.avg_boss_kill_time,
        timestamp: ts.clone(),
        fitness_formula: "zero-sum: Boss = -DirectorUtility; Director = +DirectorUtility \
            (time 600 + kill 400 + damage 200 + altar uptime 150 + economy 50 - structure reward: 8/altar, 25/obelisk)"
            .to_string(),
    };

    if let Ok(stats_json) = serde_json::to_vec_pretty(&stats) {
        let _ = atomic_write_bytes(std::path::Path::new("models/training_log.json"), &stats_json);
    }

    let manifest = GenerationManifest {
        generation_id: generations_total,
        model_format_version: MODEL_FORMAT_VERSION,
        sensor_schema_dim: TOTAL_SENSOR_SCHEMA_DIM,
        run_seed,
        boss_model_hash: boss_hash,
        director_model_hash: dir_hash,
        control_league_digest: league_report.map(|r| format!("{:#018x}", r.digest)),
        boss_control_fitness: league_report.map(|r| r.boss.fitness_mean),
        boss_control_win_rate: league_report.map(|r| r.boss.win_rate),
        director_control_fitness: league_report.map(|r| r.director.fitness_mean),
        director_control_win_rate: league_report.map(|r| r.director.win_rate),
        timestamp: ts,
    };
    if let Ok(manifest_json) = serde_json::to_vec_pretty(&manifest) {
        let _ = atomic_write_bytes(std::path::Path::new("models/generation_manifest.json"), &manifest_json);
    }

    if is_final {
        println!("[ГОТОВО] Обучение завершено. Модели и generation_manifest.json сохранены атомарно.\n");
        append_to_history_log("[ГОТОВО] Обучение завершено. Модели сохранены на диск.\n\n");
    }
}

/// Выполняет синхронизацию чемпионов в Островной Модели (Island Model).
pub fn process_island_migration(
    island_config: &IslandConfig,
    current_gen_num: u32,
    current_best_boss: &BossModel,
    current_best_dir: &DirectorModel,
    boss_pop: &mut [BossModel],
    director_pop: &mut [DirectorModel],
    hof_bosses: &mut Vec<BossModel>,
    hof_directors: &mut Vec<DirectorModel>,
) {
    let Some(ref exchange_dir) = island_config.exchange_dir else {
        return;
    };
    if let Err(e) = std::fs::create_dir_all(exchange_dir) {
        eprintln!("[ОСТРОВ '{}'] Ошибка создания папки обмена {:?}: {}", island_config.island_name, exchange_dir, e);
        return;
    }

    // 1. Атомарный экспорт лучшего Босса и Директора текущего острова
    let boss_export_path = exchange_dir.join(format!("island_{}_boss.json", island_config.island_name));
    let dir_export_path = exchange_dir.join(format!("island_{}_director.json", island_config.island_name));

    if let Ok(json) = serde_json::to_string_pretty(current_best_boss) {
        let tmp = exchange_dir.join(format!("island_{}_boss.tmp", island_config.island_name));
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &boss_export_path);
        }
    }
    if let Ok(json) = serde_json::to_string_pretty(current_best_dir) {
        let tmp = exchange_dir.join(format!("island_{}_director.tmp", island_config.island_name));
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &dir_export_path);
        }
    }

    // 2. Сканирование и импорт чемпионов с других островов
    let mut imported_bosses = 0;
    let mut imported_directors = 0;
    let my_prefix = format!("island_{}_", island_config.island_name);

    if let Ok(entries) = std::fs::read_dir(exchange_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !file_name.starts_with("island_") || !file_name.ends_with(".json") {
                continue;
            }
            // Пропускаем свои собственные файлы
            if file_name.starts_with(&my_prefix) {
                continue;
            }

            if file_name.ends_with("_boss.json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut imported_boss) = serde_json::from_str::<BossModel>(&content) {
                        let net_ok = imported_boss.neural_net.as_ref().map_or(false, |n| n.is_compatible());
                        if net_ok {
                            imported_boss.fitness_version = MODEL_FORMAT_VERSION;
                            // Добавляем иммигранта в Зал Славы Лиги
                            if hof_bosses.len() < 16 {
                                hof_bosses.push(imported_boss.clone());
                            } else {
                                let idx = rand::gen_range(1, hof_bosses.len());
                                hof_bosses[idx] = imported_boss.clone();
                            }
                            // Вводим свежую кровь в популяцию вместо худшего агента
                            if !boss_pop.is_empty() {
                                let last = boss_pop.len() - 1;
                                boss_pop[last] = imported_boss;
                            }
                            imported_bosses += 1;
                        }
                    }
                }
            } else if file_name.ends_with("_director.json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut imported_dir) = serde_json::from_str::<DirectorModel>(&content) {
                        let net_ok = imported_dir.neural_net.as_ref().map_or(false, |n| n.is_compatible());
                        if net_ok {
                            imported_dir.fitness_version = MODEL_FORMAT_VERSION;
                            // Добавляем иммигранта в Зал Славы Лиги
                            if hof_directors.len() < 16 {
                                hof_directors.push(imported_dir.clone());
                            } else {
                                let idx = rand::gen_range(1, hof_directors.len());
                                hof_directors[idx] = imported_dir.clone();
                            }
                            // Вводим свежую кровь в популяцию вместо худшего агента
                            if !director_pop.is_empty() {
                                let last = director_pop.len() - 1;
                                director_pop[last] = imported_dir;
                            }
                            imported_directors += 1;
                        }
                    }
                }
            }
        }
    }

    println!(
        "[ОСТРОВ '{}'] Миграция #{}: Экспорт чемпиона (Фитнес Boss: {:+.1}, Dir: {:+.1}) | Импорт из других островов: {} Боссов, {} Директоров",
        island_config.island_name, current_gen_num, current_best_boss.fitness, current_best_dir.fitness, imported_bosses, imported_directors
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_island_migration_roundtrip() {
        let exchange_dir = std::env::temp_dir().join("anti_vs_test_exchange");
        let _ = std::fs::remove_dir_all(&exchange_dir);
        let _ = std::fs::create_dir_all(&exchange_dir);

        let island_a = IslandConfig {
            island_name: "island_alpha".to_string(),
            exchange_dir: Some(exchange_dir.clone()),
            migrate_every: 5,
        };
        let island_b = IslandConfig {
            island_name: "island_beta".to_string(),
            exchange_dir: Some(exchange_dir.clone()),
            migrate_every: 5,
        };

        let mut boss_a = BossModel::default();
        boss_a.neural_net = Some(crate::ai::neural_net::NeuralNet::new());
        boss_a.fitness = 420.0;
        let mut dir_a = DirectorModel::default();
        dir_a.neural_net = Some(crate::ai::neural_net::DirectorBrainNet::new());
        dir_a.fitness = 150.0;

        let mut pop_boss_a = vec![boss_a.clone(), boss_a.clone()];
        let mut pop_dir_a = vec![dir_a.clone(), dir_a.clone()];
        let mut hof_boss_a = vec![boss_a.clone()];
        let mut hof_dir_a = vec![dir_a.clone()];

        // Миграция острова А: экспорт
        process_island_migration(
            &island_a,
            10,
            &boss_a,
            &dir_a,
            &mut pop_boss_a,
            &mut pop_dir_a,
            &mut hof_boss_a,
            &mut hof_dir_a,
        );

        let exported_boss_a = exchange_dir.join("island_island_alpha_boss.json");
        let exported_dir_a = exchange_dir.join("island_island_alpha_director.json");
        assert!(exported_boss_a.exists(), "Файл экспорта босса острова А должен существовать");
        assert!(exported_dir_a.exists(), "Файл экспорта директора острова А должен существовать");

        // Миграция острова Б: импорт от острова А и экспорт чемпиона острова Б
        let mut boss_b = BossModel::default();
        boss_b.neural_net = Some(crate::ai::neural_net::NeuralNet::new());
        boss_b.fitness = 300.0;
        let mut dir_b = DirectorModel::default();
        dir_b.neural_net = Some(crate::ai::neural_net::DirectorBrainNet::new());
        dir_b.fitness = 200.0;

        let mut pop_boss_b = vec![boss_b.clone(), boss_b.clone()];
        let mut pop_dir_b = vec![dir_b.clone(), dir_b.clone()];
        let mut hof_boss_b = vec![boss_b.clone()];
        let mut hof_dir_b = vec![dir_b.clone()];

        process_island_migration(
            &island_b,
            10,
            &boss_b,
            &dir_b,
            &mut pop_boss_b,
            &mut pop_dir_b,
            &mut hof_boss_b,
            &mut hof_dir_b,
        );

        // Проверяем, что в Зал Славы острова Б попал чемпион острова А
        assert_eq!(hof_boss_b.len(), 2, "Зал славы должен содержать 2 боссов после импорта");
        assert_eq!(hof_boss_b[1].fitness, 420.0);
        assert_eq!(hof_dir_b.len(), 2, "Зал славы должен содержать 2 директоров после импорта");
        assert_eq!(hof_dir_b[1].fitness, 150.0);

        let _ = std::fs::remove_dir_all(&exchange_dir);
    }

    #[test]
    fn test_seeded_coevolution_reproducible_and_atomic_write() {
        let cfg = IslandConfig::default();
        let a = train_co_evolution_seed(1, 3, true, Some(30.0), 0.2, 1, &cfg, 0x7777_u64, false);
        let b = train_co_evolution_seed(1, 3, true, Some(30.0), 0.2, 1, &cfg, 0x7777_u64, false);

        assert_eq!(a.best_boss.fitness, b.best_boss.fitness);
        assert_eq!(a.best_director.fitness, b.best_director.fitness);
        assert_eq!(
            serde_json::to_vec(&a.best_boss).unwrap(),
            serde_json::to_vec(&b.best_boss).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&a.best_director).unwrap(),
            serde_json::to_vec(&b.best_director).unwrap()
        );

        let tmp_dir = std::env::temp_dir().join("anti_vs_test_atomic");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let target = tmp_dir.join("manifest.json");
        atomic_write_bytes(&target, br#"{"ok":true}"#).expect("atomic write succeeds");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), r#"{"ok":true}"#);
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
