use crate::core::rand;
use rayon::prelude::*;
use crate::ai::models::{BossModel, DirectorModel, TrainingStats, MODEL_FORMAT_VERSION};
use crate::ai::simulation::{run_headless_simulation, MatchResult};
use crate::ecs::components::inventory::{PassiveType, WeaponType};

const MATCHES_PER_AGENT: usize = 3;
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
/// - -100 за уровень Босса (чтобы Директор не «кормил» его опытом).
fn director_utility(res: &MatchResult) -> f32 {
    let cap = res.match_cap.max(1.0);
    let survival_frac = (res.survival_time / cap).clamp(0.0, 1.0);
    let time_reward = (1.0 - survival_frac) * 600.0;
    let kill_bonus = if res.boss_won { 0.0 } else { 400.0 };
    let damage_frac = (res.damage_dealt_to_boss / res.boss_max_health.max(1.0)).clamp(0.0, 1.0);
    let damage_reward = damage_frac * 200.0;
    let uptime_denom = res.survival_time.max(1.0) * res.altars_built.max(1) as f32;
    let uptime_frac = (res.total_altar_uptime_seconds / uptime_denom).clamp(0.0, 1.0);
    let altar_reward = uptime_frac * 150.0;
    let economy_denom = res.director_points_spent + res.director_points_unspent + 100.0;
    let economy_reward =
        (res.director_points_spent / economy_denom).clamp(0.0, 1.0) * 50.0;
    let level_penalty = (res.boss_level as f32 / 50.0).clamp(0.0, 1.0) * 100.0;

    (time_reward + kill_bonus + damage_reward + altar_reward + economy_reward - level_penalty)
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

pub fn run_co_evolutionary_training(generations: usize, population_size: usize, reset: bool) {
    let generations = generations.max(1);
    let population_size = population_size.max(2);
    println!("============================================================");
    println!(">>> ЗАПУСК ТУРНИРА СО-ЭВОЛЮЦИИ ИИ (LEAGUE SELF-PLAY)");
    println!("  Движок: {} потоков Rayon (ноль-суммарные матчи с фиксированными seed)", rayon::current_num_threads());
    println!("  Поколений: {} | Размер популяции каждого агента: {}", generations, population_size);
    println!("  Формат: v{} (старый фитнес инвалидируется при загрузке)", MODEL_FORMAT_VERSION);
    println!("  Зал Славы (League Hall of Fame): АКТИВЕН (защита от забывания)");
    if reset {
        println!("  [СБРОС --clean]: ИИ Босса очищен, обучение начнется с нуля!");
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
        }
        if let Some(ref net) = director.neural_net {
            director.total_synapses = net.total_synapses();
        }
        if director.fitness_version != MODEL_FORMAT_VERSION || !director.fitness.is_finite() {
            director.fitness = f32::NEG_INFINITY;
        }
        director.model_version = MODEL_FORMAT_VERSION;
        director.fitness_version = MODEL_FORMAT_VERSION;

        let base = boss.generation;
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
                m.mob_repulsion_weight = (m.mob_repulsion_weight + rand::gen_range(-30.0, 30.0)).clamp(250.0, 700.0);
                m.gem_attraction_weight = (m.gem_attraction_weight + rand::gen_range(-25.0, 25.0)).clamp(100.0, 400.0);
                m.sniper_hunt_aggression = (m.sniper_hunt_aggression + rand::gen_range(-30.0, 30.0)).clamp(150.0, 500.0);
                m.altar_siege_aggression = (m.altar_siege_aggression + rand::gen_range(-25.0, 25.0)).clamp(150.0, 450.0);
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
                d.altar_build_priority = (d.altar_build_priority + rand::gen_range(-0.10, 0.10)).clamp(0.1, 1.0);
                for w in d.squad_spawn_weights.iter_mut() {
                    *w = (*w + rand::gen_range(-0.15, 0.15)).max(0.1);
                }
            }
            d
        })
        .collect();

    // Никаких фиктивных нулей: NEG_INFINITY заставляет первого же реального
    // чемпиона стать лучшим, даже если загруженная модель была слабой.
    let mut best_overall_boss = seed_boss.clone();
    let mut best_overall_director = seed_director.clone();
    let mut best_avg_boss_level = 0u32;
    let mut total_matches_played = 0usize;
    let mut boss_sigma = 0.20f32;
    let mut director_sigma = 0.18f32;
    let mut prev_best_boss_fitness = best_overall_boss.fitness;
    let mut prev_best_director_fitness = best_overall_director.fitness;
    let base_seed = system_base_seed();

    // 3. Цикл поколений обучения
    for gen_idx in 1..=generations {
        let current_gen_num = base_gen + gen_idx as u32;

        // Curriculum: ранние поколения короткие, поздние — полные 17 минут.
        let progress = if generations > 1 {
            (gen_idx - 1) as f32 / (generations - 1) as f32
        } else {
            1.0
        };
        let curriculum_start = if base_gen > 0 { FULL_MATCH_CAP } else { START_MATCH_CAP };
        let match_cap = ((curriculum_start
            + (FULL_MATCH_CAP - curriculum_start) * progress)
            / 30.0)
            .round()
            * 30.0;

        // 3.1 Формируем общую матрицу матчей: каждый агент играет ровно
        // MATCHES_PER_AGENT матчей в лиге + один матч против чемпиона Зала Славы.
        let mut pairings: Vec<Pairing> = Vec::with_capacity(population_size * (MATCHES_PER_AGENT + 2));
        let mut boss_order: Vec<usize> = (0..population_size).collect();
        let mut director_order: Vec<usize> = (0..population_size).collect();

        for round in 0..MATCHES_PER_AGENT {
            shuffle(&mut boss_order);
            shuffle(&mut director_order);
            for i in 0..population_size {
                let seed = base_seed
                    .wrapping_add((gen_idx as u64) << 40)
                    .wrapping_add((round as u64) << 32)
                    .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                pairings.push(Pairing {
                    boss: AgentRef::Population(boss_order[i]),
                    director: AgentRef::Population(director_order[i]),
                    seed,
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
                run_headless_simulation(boss, director, p.seed, match_cap)
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

        let current_best_boss = &boss_pop[0];
        let current_best_dir = &director_pop[0];

        if current_best_boss.fitness > best_overall_boss.fitness {
            best_overall_boss = current_best_boss.clone();
        }
        if current_best_dir.fitness > best_overall_director.fitness {
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
        if best_boss_eval.fitness <= prev_best_boss_fitness + 0.5 {
            boss_sigma = (boss_sigma * 1.08).min(0.60);
        } else {
            boss_sigma = (boss_sigma * 0.92).max(0.06);
        }
        prev_best_boss_fitness = prev_best_boss_fitness.max(best_boss_eval.fitness);

        if best_dir_eval.fitness <= prev_best_director_fitness + 0.5 {
            director_sigma = (director_sigma * 1.08).min(0.60);
        } else {
            director_sigma = (director_sigma * 0.92).max(0.06);
        }
        prev_best_director_fitness = prev_best_director_fitness.max(best_dir_eval.fitness);

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
            "    * Разрушено алтарей: {} | Скиллы ({}): [{}]",
            best_boss_eval.avg_altars_destroyed,
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
            "    ↳ Противник (Босс): Уровень {} | Пешком {:.0} px [Рывков {}] | Медитация {:.1} мин | Разрушил алтарей {}",
            best_dir_eval.avg_boss_level,
            best_dir_eval.avg_boss_distance_walked,
            best_dir_eval.avg_boss_dashes,
            best_dir_eval.avg_boss_meditation / 60.0,
            best_dir_eval.avg_altars_destroyed
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

        // 4. Селекция: элита + турнир + кроссовер + мутация + weight decay
        let mut next_boss_pop = vec![boss_pop[0].clone(), boss_pop[1].clone()];
        let boss_fitness: Vec<f32> = boss_pop.iter().map(|m| m.fitness).collect();

        while next_boss_pop.len() < population_size {
            let p1_idx = tournament_index(&boss_fitness, 3);
            let p2_idx = tournament_index(&boss_fitness, 3);
            let p1 = &boss_pop[p1_idx];
            let p2 = &boss_pop[p2_idx];
            let mut child = p1.clone();

            if let (Some(net1), Some(net2)) = (&p1.neural_net, &p2.neural_net) {
                let mut child_net = net1.crossover(net2);
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

            child.mob_repulsion_weight = (p1.mob_repulsion_weight + p2.mob_repulsion_weight) * 0.5;
            child.gem_attraction_weight = (p1.gem_attraction_weight + p2.gem_attraction_weight) * 0.5;
            child.altar_siege_aggression = (p1.altar_siege_aggression + p2.altar_siege_aggression) * 0.5;
            child.sniper_hunt_aggression = (p1.sniper_hunt_aggression + p2.sniper_hunt_aggression) * 0.5;

            child.mob_repulsion_weight = (child.mob_repulsion_weight + rand::gen_range(-25.0, 25.0)).clamp(250.0, 700.0);
            child.gem_attraction_weight = (child.gem_attraction_weight + rand::gen_range(-20.0, 20.0)).clamp(100.0, 400.0);
            child.altar_siege_aggression = (child.altar_siege_aggression + rand::gen_range(-25.0, 25.0)).clamp(150.0, 500.0);
            child.sniper_hunt_aggression = (child.sniper_hunt_aggression + rand::gen_range(-30.0, 30.0)).clamp(150.0, 500.0);
            child.dash_danger_threshold = (child.dash_danger_threshold as i32 + rand::gen_range(-1, 2)).clamp(8, 18) as usize;

            for w in child.weapon_priorities.iter_mut() {
                if rand::gen_range(0.0, 1.0) < 0.25 {
                    *w = (*w + rand::gen_range(-0.25, 0.25)).clamp(0.2, 3.5);
                }
            }
            for p in child.passive_priorities.iter_mut() {
                if rand::gen_range(0.0, 1.0) < 0.25 {
                    *p = (*p + rand::gen_range(-0.25, 0.25)).clamp(0.2, 3.5);
                }
            }
            for a in child.combo_archetype_weights.iter_mut() {
                if rand::gen_range(0.0, 1.0) < 0.30 {
                    *a = (*a + rand::gen_range(-0.30, 0.30)).clamp(0.2, 4.0);
                }
            }
            child.synergy_seeking_weight = (child.synergy_seeking_weight + rand::gen_range(-0.15, 0.15)).clamp(0.5, 3.5);

            next_boss_pop.push(child);
        }

        let mut next_dir_pop = vec![director_pop[0].clone(), director_pop[1].clone()];
        let director_fitness: Vec<f32> = director_pop.iter().map(|m| m.fitness).collect();

        while next_dir_pop.len() < population_size {
            let p1_idx = tournament_index(&director_fitness, 3);
            let p2_idx = tournament_index(&director_fitness, 3);
            let p1 = &director_pop[p1_idx];
            let p2 = &director_pop[p2_idx];
            let mut child = p1.clone();

            if let (Some(net1), Some(net2)) = (&p1.neural_net, &p2.neural_net) {
                let mut crossed_net = net1.crossover(net2);
                crossed_net.mutate(0.08, director_sigma);
                crossed_net.decay(0.002);
                child.total_synapses = crossed_net.total_synapses();
                child.neural_net = Some(crossed_net);
            } else {
                let mut net = crate::ai::neural_net::DirectorBrainNet::new();
                net.mutate(0.10, director_sigma);
                child.total_synapses = net.total_synapses();
                child.neural_net = Some(net);
            }

            child.altar_build_priority = ((p1.altar_build_priority + p2.altar_build_priority) * 0.5
                + rand::gen_range(-0.08, 0.08))
            .clamp(0.1, 1.0);
            for i in 0..child.squad_spawn_weights.len() {
                let p1_w = p1.squad_spawn_weights.get(i).copied().unwrap_or(1.0);
                let p2_w = p2.squad_spawn_weights.get(i).copied().unwrap_or(1.0);
                child.squad_spawn_weights[i] =
                    ((p1_w + p2_w) * 0.5 + rand::gen_range(-0.15, 0.15)).max(0.1);
            }

            next_dir_pop.push(child);
        }

        boss_pop = next_boss_pop;
        director_pop = next_dir_pop;

        // Авто-сохранение каждые 25 поколений
        if gen_idx % 25 == 0 && gen_idx < generations {
            save_models(
                &best_overall_boss,
                &best_overall_director,
                base_gen as usize + gen_idx,
                total_matches_played,
                best_avg_boss_level,
                false,
            );
        }
    }

    // 5. Финальный экспорт
    save_models(
        &best_overall_boss,
        &best_overall_director,
        base_gen as usize + generations,
        total_matches_played,
        best_avg_boss_level,
        true,
    );
}

fn save_models(
    boss: &BossModel,
    director: &DirectorModel,
    generations_total: usize,
    matches_played: usize,
    best_boss_level: u32,
    is_final: bool,
) {
    let _ = std::fs::create_dir_all("models");
    if is_final {
        println!("\n[СОХРАНЕНИЕ] Финальный экспорт обученных моделей...");
    } else {
        println!("\n[ЧЕКПОИНТ #{}] Автоматическое сохранение прогресса на диск...", boss.generation);
    }

    // Гарантируем корректные метаданные формата перед записью
    let mut boss_out = boss.clone();
    boss_out.model_version = MODEL_FORMAT_VERSION;
    boss_out.fitness_version = MODEL_FORMAT_VERSION;
    boss_out.version = format!("v{}.0-end-to-end", MODEL_FORMAT_VERSION);
    if let Some(ref net) = boss_out.neural_net {
        boss_out.total_synapses = net.total_synapses();
    }

    let mut director_out = director.clone();
    director_out.model_version = MODEL_FORMAT_VERSION;
    director_out.fitness_version = MODEL_FORMAT_VERSION;
    director_out.version = format!("v{}.0-end-to-end", MODEL_FORMAT_VERSION);
    if let Some(ref net) = director_out.neural_net {
        director_out.total_synapses = net.total_synapses();
    }

    if let Ok(boss_json) = serde_json::to_string_pretty(&boss_out) {
        if std::fs::write("models/boss_policy.json", &boss_json).is_ok() {
            println!(
                "  [OK] Модель Босса ({} синапсов, Поколение #{}) -> models/boss_policy.json",
                boss_out.total_synapses, boss_out.generation
            );
        }
    }

    if let Some(ref net) = boss_out.neural_net {
        if let Ok(brain_json) = serde_json::to_string_pretty(net) {
            let _ = std::fs::write("models/boss_brain_neural.json", brain_json);
            println!("  [OK] Матрица синапсов Босса (150 -> 64 -> 48 -> 7) -> models/boss_brain_neural.json");
        }
    }

    let archetype_names = [
        "Инфернальный Пиромант (Огонь, Пламя, Сердце Пироманта)",
        "Сверхпроводящий Громовержец (Молния, Тесла, Лира, Клевер)",
        "Несокрушимый Бастион (Орбиты, Молот, Щиты, Эгида)",
        "Аннигилятор Бездны (Луч Смерти, Сингулярность, Око Колосса)",
        "Кровавый Жнец (Топор, Кинжалы, Вампиризм, Клевер)",
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

    if let Ok(dir_json) = serde_json::to_string_pretty(&director_out) {
        if std::fs::write("models/director_policy.json", &dir_json).is_ok() {
            println!(
                "  [OK] Модель Директора ({} синапсов, Поколение #{}) -> models/director_policy.json",
                director_out.total_synapses, director_out.generation
            );
        }
    }

    if let Some(ref net) = director_out.neural_net {
        if let Ok(dir_brain_json) = serde_json::to_string_pretty(net) {
            let _ = std::fs::write("models/director_brain_neural.json", dir_brain_json);
            println!("  [OK] Матрица синапсов Директора (132 -> 64 -> 48; 20+3+8) -> models/director_brain_neural.json");
        }
    }

    let stats = TrainingStats {
        total_generations: generations_total,
        total_matches_played: matches_played,
        best_boss_survival_time: boss_out.avg_survival_time,
        best_boss_level,
        best_director_kill_time: director_out.avg_boss_kill_time,
        timestamp: utc_timestamp(),
        fitness_formula: "zero-sum: Boss = -DirectorUtility; Director = +DirectorUtility \
            (time 600 + kill 400 + damage 200 + altar uptime 150 + economy 50 - boss level 100)"
            .to_string(),
    };

    if let Ok(stats_json) = serde_json::to_string_pretty(&stats) {
        let _ = std::fs::write("models/training_log.json", stats_json);
    }

    if is_final {
        println!("[ГОТОВО] Обучение завершено. Модели сохранены и готовы к игре.\n");
        append_to_history_log("[ГОТОВО] Обучение завершено. Модели сохранены на диск.\n\n");
    }
}
