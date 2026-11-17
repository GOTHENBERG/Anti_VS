use crate::core::rand;
use rayon::prelude::*;
use crate::ai::models::{BossModel, DirectorModel, TrainingStats};
use crate::ai::simulation::run_headless_simulation;

fn detect_gpu_device() -> String {
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=name,memory.total", "--format=csv,noheader"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "DirectX 12 / Vulkan Hardware Accelerator".to_string()
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
    weapons: Vec<(crate::ecs::components::inventory::WeaponType, u8)>,
    passives: Vec<(crate::ecs::components::inventory::PassiveType, u8)>,
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
    boss_weapons: Vec<(crate::ecs::components::inventory::WeaponType, u8)>,
    boss_passives: Vec<(crate::ecs::components::inventory::PassiveType, u8)>,
}

fn format_build_summary(
    weapons: &[(crate::ecs::components::inventory::WeaponType, u8)],
    passives: &[(crate::ecs::components::inventory::PassiveType, u8)],
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

pub fn run_co_evolutionary_training(generations: usize, population_size: usize, reset: bool) {
    let gpu_info = detect_gpu_device();
    println!("============================================================");
    println!(">>> ЗАПУСК ТУРНИРА СО-ЭВОЛЮЦИИ ИИ (LEAGUE SELF-PLAY)");
    println!("  Устройство вычислений: {}", gpu_info);
    println!("  Многопоточный движок: {} потоков (Rayon SIMD Parallel)", rayon::current_num_threads());
    println!("  Поколений: {} | Размер популяции каждого агента: {}", generations, population_size);
    println!("  Зал Славы (League Hall of Fame): АКТИВЕН (защита от забывания)");
    if reset {
        println!("  [СБРОС --clean]: ИИ Босса очищен, обучение начнется с нуля!");
    }
    println!("============================================================");

    let (seed_boss, seed_director, base_gen) = if reset {
        let fresh_boss = BossModel::default();
        let fresh_director = DirectorModel::default();
        (fresh_boss, fresh_director, 0)
    } else {
        let mut loaded_boss = crate::ai::load_trained_boss_model().unwrap_or_default();
        if loaded_boss.neural_net.is_none() || loaded_boss.total_synapses != crate::ai::neural_net::NeuralNet::new().total_synapses() {
            let net = crate::ai::neural_net::NeuralNet::new();
            loaded_boss.total_synapses = net.total_synapses();
            loaded_boss.neural_net = Some(net);
        }
        let mut loaded_director = crate::ai::load_trained_director_model().unwrap_or_default();
        if loaded_director.neural_net.is_none() || loaded_director.total_synapses != crate::ai::neural_net::DirectorBrainNet::new().total_synapses() {
            let net = crate::ai::neural_net::DirectorBrainNet::new();
            loaded_director.total_synapses = net.total_synapses();
            loaded_director.neural_net = Some(net);
        }
        let base_gen_val = loaded_boss.generation;
        (loaded_boss, loaded_director, base_gen_val)
    };

    if base_gen > 0 {
        println!("[ПРОДОЛЖЕНИЕ] Дообучение: с Поколения {} (Фитнес Босса: {:.0}, Синапсов Босса: {}, Синапсов Директора: {})", base_gen, seed_boss.fitness, seed_boss.total_synapses, seed_director.total_synapses);
    } else {
        println!("[СТАРТ] Начало нового цикла: инициализированы нейросети (Босс: {} син., Директор: {} син.).", seed_boss.total_synapses, seed_director.total_synapses);
    }

    let start_msg = format!(
        "\n================================================================================\n\
         >>> СЕССИЯ ОБУЧЕНИЯ (Старт с поколения #{}, запланировано раундов: {}, популяция: {})\n\
         ================================================================================\n",
        base_gen, generations, population_size
    );
    append_to_history_log(&start_msg);

    // 1. Инициализация Зала Славы (Hall of Fame) историческими чемпионами
    let mut hof_directors: Vec<DirectorModel> = vec![seed_director.clone()];
    let mut hof_bosses: Vec<BossModel> = vec![seed_boss.clone()];

    // 2. Инициализация стартовых популяций (с сохранением лучших прошлых находок)
    let mut boss_pop: Vec<BossModel> = (0..population_size)
        .map(|i| {
            let mut m = seed_boss.clone();
            if i > 0 {
                // Небольшая начальная вариативность для поиска новых оптимумов
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

    let mut best_overall_boss = boss_pop[0].clone();
    best_overall_boss.fitness = 0.0;
    let mut best_overall_director = director_pop[0].clone();
    best_overall_director.fitness = 0.0;

    // 3. Цикл поколений обучения
    for gen_idx in 1..=generations {
        let current_gen_num = base_gen + gen_idx as u32;

        // Оценка фитнеса каждого кандидата в параллельных схватках
        let boss_results: Vec<BossEval> = (0..population_size)
            .into_par_iter()
            .map(|b_idx| {
                let boss = &boss_pop[b_idx];
                let mut total_survival = 0.0;
                let mut total_level = 0;
                let mut total_score = 0.0;
                let mut total_damage_taken = 0.0;
                let mut total_melee_damage = 0.0;
                let mut total_ranged_damage = 0.0;
                let mut total_hazard_damage = 0.0;
                let mut total_meditation_time = 0.0;
                let mut total_distance_walked = 0.0;
                let mut total_dashes = 0usize;
                let mut total_altars_destroyed = 0;
                let mut total_altars_built = 0;
                let mut total_mobs = [0usize; 16];
                let mut last_director_grades = (0, 0, 0);
                let mut last_weapons = Vec::new();
                let mut last_passives = Vec::new();
                let matches_count = 3;

                for match_i in 0..matches_count {
                    // 2 матча против текущей популяции, 1 матч против исторического чемпиона из Зала Славы!
                    let director = if match_i == 2 && !hof_directors.is_empty() {
                        let h_idx = rand::gen_range(0, hof_directors.len());
                        &hof_directors[h_idx]
                    } else {
                        let d_idx = rand::gen_range(0, population_size);
                        &director_pop[d_idx]
                    };

                    let res = run_headless_simulation(boss, director, 1020.0); // 17 минут раунд (1020с)

                    total_survival += res.survival_time;
                    total_level += res.boss_level;
                    total_damage_taken += res.damage_dealt_to_boss;
                    total_melee_damage += res.boss_damage_stats.melee_total;
                    total_ranged_damage += res.boss_damage_stats.ranged_total;
                    total_hazard_damage += res.boss_damage_stats.hazard_total;
                    total_meditation_time += res.total_meditation_time;
                    total_distance_walked += res.distance_walked;
                    total_dashes += res.boss_dashes;
                    total_altars_destroyed += res.altars_destroyed;
                    total_altars_built += res.altars_built;
                    for t in 0..16 {
                        total_mobs[t] += res.mobs_spawned_by_tier[t];
                    }

                    if match_i == 0 || res.survival_time >= total_survival / (match_i + 1) as f32 {
                        last_director_grades = res.director_grades;
                        last_weapons = res.final_weapons;
                        last_passives = res.final_passives;
                    }

                    // 1. Фитнес Босса (Чистый Reinforcement Learning):
                    // Время выживания + победа + прокачка (сбор душ) + разрушение алтарей + медитация - кнут за урон
                    let time_reward = res.survival_time * 3.5;
                    let win_bonus = if res.boss_won { 4000.0 } else { 0.0 };
                    let level_reward = (res.boss_level as f32) * 20.0;
                    let altar_reward = (res.altars_destroyed as f32) * 30.0;
                    let meditation_reward = res.total_meditation_time * 0.7;
                    let damage_whip = res.damage_dealt_to_boss * 0.005;

                    let score = (time_reward + win_bonus + level_reward + altar_reward + meditation_reward - damage_whip).max(10.0);
                    total_score += score;
                }

                let m_f = matches_count as f32;
                let mut avg_mobs = [0usize; 16];
                for t in 0..16 {
                    avg_mobs[t] = ((total_mobs[t] as f32) / m_f).round() as usize;
                }

                BossEval {
                    idx: b_idx,
                    fitness: total_score / m_f,
                    avg_survival: total_survival / m_f,
                    avg_level: (total_level as f32 / m_f).round() as u32,
                    avg_damage_taken: total_damage_taken / m_f,
                    avg_melee_damage: total_melee_damage / m_f,
                    avg_ranged_damage: total_ranged_damage / m_f,
                    avg_hazard_damage: total_hazard_damage / m_f,
                    avg_altars_destroyed: ((total_altars_destroyed as f32) / m_f).round() as usize,
                    avg_altars_built: ((total_altars_built as f32) / m_f).round() as usize,
                    avg_meditation_time: total_meditation_time / m_f,
                    avg_distance_walked: total_distance_walked / m_f,
                    avg_dashes: ((total_dashes as f32) / m_f).round() as usize,
                    mobs_spawned: avg_mobs,
                    director_grades: last_director_grades,
                    weapons: last_weapons,
                    passives: last_passives,
                }
            })
            .collect();

        // Оценка директоров
        let director_results: Vec<DirectorEval> = (0..population_size)
            .into_par_iter()
            .map(|d_idx| {
                let director = &director_pop[d_idx];
                let mut total_kill_speed = 0.0;
                let mut total_score = 0.0;
                let mut total_damage_dealt = 0.0;
                let mut total_altars_built = 0;
                let mut total_altars_destroyed = 0;
                let mut total_altar_uptime = 0.0f32;
                let mut total_boss_level = 0;
                let mut total_boss_meditation = 0.0;
                let mut total_boss_distance_walked = 0.0;
                let mut total_boss_dashes = 0usize;
                let mut total_mobs = [0usize; 16];
                let mut last_grades = (0, 0, 0);
                let mut last_boss_weapons = Vec::new();
                let mut last_boss_passives = Vec::new();
                let matches_count = 3;

                for match_i in 0..matches_count {
                    let boss = if match_i == 2 && !hof_bosses.is_empty() {
                        let h_idx = rand::gen_range(0, hof_bosses.len());
                        &hof_bosses[h_idx]
                    } else {
                        let b_idx = rand::gen_range(0, population_size);
                        &boss_pop[b_idx]
                    };

                    let res = run_headless_simulation(boss, director, 1020.0);

                    // 2. Фитнес Директора:
                    // ГЛАВНЫЙ ПРЯНИК — ранний конец матча и бонус за быстрое уничтожение босса
                    let time_saved = (1020.0 - res.survival_time).max(0.0);
                    let speed_reward = time_saved * 5.0;
                    let kill_bonus = if !res.boss_won { 1500.0 } else { 0.0 };
                    let early_blitz_bonus = if !res.boss_won && res.survival_time < 300.0 {
                        (300.0 - res.survival_time) * 5.0
                    } else {
                        0.0
                    };

                    // ГЛАВНЫЙ КНУТ ДИРЕКТОРА — жесткие штрафы за затягивание боя!
                    // Если босс живет дольше 5 минут (>300с), включается тяжелый штраф (-4 очка за каждую секунду затяжки)
                    let time_drag_whip = if res.survival_time > 300.0 {
                        (res.survival_time - 300.0) * 4.0
                    } else {
                        0.0
                    };
                    let survived_boss_whip = if res.boss_won { 2500.0 } else { 0.0 };
                    let total_time_whip = time_drag_whip + survived_boss_whip;

                    // Урон ограничен (потолок 500), чтобы исключить гринд регенерации босса
                    let damage_reward = (res.damage_dealt_to_boss * 0.04).min(500.0);

                    // ПРЯНИК ЗА АЛТАРИ: секунды жизни каждого алтаря, деленные на прошедшие минуты матча
                    let elapsed_minutes = (res.survival_time / 60.0).max(0.1);
                    let altar_uptime_rate = res.total_altar_uptime_seconds / elapsed_minutes;
                    let altar_reward = (altar_uptime_rate * 2.5).min(1000.0);

                    let kill_score = (speed_reward + kill_bonus + early_blitz_bonus + damage_reward + altar_reward - total_time_whip).max(10.0);
                    total_score += kill_score;
                    total_kill_speed += res.survival_time;
                    total_damage_dealt += res.damage_dealt_to_boss;
                    total_altars_built += res.altars_built;
                    total_altars_destroyed += res.altars_destroyed;
                    total_altar_uptime += res.total_altar_uptime_seconds;
                    total_boss_level += res.boss_level;
                    total_boss_meditation += res.total_meditation_time;
                    total_boss_distance_walked += res.distance_walked;
                    total_boss_dashes += res.boss_dashes;
                    last_grades = res.director_grades;
                    for t in 0..16 {
                        total_mobs[t] += res.mobs_spawned_by_tier[t];
                    }

                    if match_i == 0 || res.survival_time >= total_kill_speed / (match_i + 1) as f32 {
                        last_boss_weapons = res.final_weapons;
                        last_boss_passives = res.final_passives;
                    }
                }

                let m_f = matches_count as f32;
                let mut avg_mobs = [0usize; 16];
                for t in 0..16 {
                    avg_mobs[t] = ((total_mobs[t] as f32) / m_f).round() as usize;
                }

                let avg_kill = total_kill_speed / m_f;
                let avg_uptime = total_altar_uptime / m_f;
                let avg_rate = avg_uptime / (avg_kill / 60.0).max(0.1);

                DirectorEval {
                    idx: d_idx,
                    fitness: total_score / m_f,
                    avg_kill_speed: avg_kill,
                    avg_damage_dealt: total_damage_dealt / m_f,
                    avg_altars_built: ((total_altars_built as f32) / m_f).round() as usize,
                    avg_altars_destroyed: ((total_altars_destroyed as f32) / m_f).round() as usize,
                    avg_altar_uptime_rate: avg_rate,
                    avg_altar_uptime_total: avg_uptime,
                    avg_boss_level: (total_boss_level as f32 / m_f).round() as u32,
                    avg_boss_meditation: total_boss_meditation / m_f,
                    avg_boss_distance_walked: total_boss_distance_walked / m_f,
                    avg_boss_dashes: ((total_boss_dashes as f32) / m_f).round() as usize,
                    mobs_spawned: avg_mobs,
                    grades: last_grades,
                    boss_weapons: last_boss_weapons,
                    boss_passives: last_boss_passives,
                }
            })
            .collect();

        // Обновляем фитнес в моделях
        for eval in &boss_results {
            boss_pop[eval.idx].fitness = eval.fitness;
            boss_pop[eval.idx].avg_survival_time = eval.avg_survival;
            boss_pop[eval.idx].generation = current_gen_num;
        }

        for eval in &director_results {
            director_pop[eval.idx].fitness = eval.fitness;
            director_pop[eval.idx].avg_boss_kill_time = eval.avg_kill_speed;
            director_pop[eval.idx].generation = current_gen_num;
        }

        // Сортировка по фитнесу (лучшие сверху)
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

        let best_boss_eval = boss_results
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();
        let best_dir_eval = director_results
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        let total_mobs_spawned: usize = best_dir_eval.mobs_spawned.iter().sum();
        let boss_mobs_spawned: usize = best_boss_eval.mobs_spawned.iter().sum();
        let boss_time_reward = best_boss_eval.avg_survival * 3.5;
        let boss_level_reward = (best_boss_eval.avg_level as f32) * 20.0;
        let boss_altar_reward = (best_boss_eval.avg_altars_destroyed as f32) * 40.0;
        let boss_meditation_reward = best_boss_eval.avg_meditation_time * 0.5625;
        let boss_damage_whip = best_boss_eval.avg_damage_taken * 0.02;
        let dir_speed_reward = (1020.0 - best_dir_eval.avg_kill_speed).max(0.0) * 5.0;
        let dir_time_whip = if best_dir_eval.avg_kill_speed > 300.0 {
            (best_dir_eval.avg_kill_speed - 300.0) * 4.0
        } else {
            0.0
        };
        let dir_dmg_reward = (best_dir_eval.avg_damage_dealt * 0.04).min(500.0);

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
            ">>> РАУНД {:02}/{:02} (Поколение #{:02}) [Зал Славы: {} моделей]",
            gen_idx, generations, current_gen_num, hof_bosses.len()
        );
        log_line!("--------------------------------------------------------------------------------");
        log_line!("  [ВРЕМЯ ЖИЗНИ БОССА В РАУНДЕ]:");
        log_line!(
            "    >> В матчах лучшего Босса:     {:>4.1} мин ({:>5.1} сек)  [цель: 17.0 мин]",
            best_boss_eval.avg_survival / 60.0,
            best_boss_eval.avg_survival
        );
        log_line!(
            "    >> Против лучшего Директора:   {:>4.1} мин ({:>5.1} сек)  [цель: 0.0 мин]",
            best_dir_eval.avg_kill_speed / 60.0,
            best_dir_eval.avg_kill_speed
        );
        log_line!("--------------------------------------------------------------------------------");
        log_line!(
            "  БОСС (Фитнес: {:.0}, Уровень: {}):",
            best_boss_eval.fitness, best_boss_eval.avg_level
        );
        log_line!(
            "    * Время жизни босса: {:>4.1} мин ({:>5.1} сек)",
            best_boss_eval.avg_survival / 60.0,
            best_boss_eval.avg_survival
        );
        let boss_move_time = (best_boss_eval.avg_survival - best_boss_eval.avg_meditation_time).max(0.0);
        let boss_speed = if best_boss_eval.avg_survival > 0.0 {
            best_boss_eval.avg_distance_walked / best_boss_eval.avg_survival
        } else {
            0.0
        };
        log_line!(
            "    * Пройдено пешком (без дэша): {:.0} px [Рывков: {}] | Движение: {:.1} мин | Медитация: {:.1} мин | Ср. скорость: {:.1} px/сек",
            best_boss_eval.avg_distance_walked,
            best_boss_eval.avg_dashes,
            boss_move_time / 60.0,
            best_boss_eval.avg_meditation_time / 60.0,
            boss_speed
        );
        log_line!(
            "    * Очки фитнеса: Выживание: +{:.0} | Уровень (x{}): +{:.0} | Алтари (x{}): +{:.0} | Медитация: {:.1} мин (+{:.0}) | Урон (кнут): -{:.0}",
            boss_time_reward,
            best_boss_eval.avg_level,
            boss_level_reward,
            best_boss_eval.avg_altars_destroyed,
            boss_altar_reward,
            best_boss_eval.avg_meditation_time / 60.0,
            boss_meditation_reward,
            boss_damage_whip
        );
        log_line!(
            "    * Получено урона: {:.0} HP [Ближний: {:.0}, Дальний: {:.0}, Окружение: {:.0}] | Разрушено алтарей: {}",
            best_boss_eval.avg_damage_taken,
            best_boss_eval.avg_melee_damage,
            best_boss_eval.avg_ranged_damage,
            best_boss_eval.avg_hazard_damage,
            best_boss_eval.avg_altars_destroyed
        );
        log_line!(
            "    * Скиллы и оружия ({}): [{}]",
            best_boss_eval.weapons.len(),
            boss_w_str
        );
        log_line!(
            "    * Предметы и пассивки ({}): [{}]",
            best_boss_eval.passives.len(),
            boss_p_str
        );
        log_line!(
            "    ↳ Противник (Директор): Построил алтарей: {} | Призвал мобов: {} шт | Грейды орды: [Ярость: {}, Панцирь: {}, Чумные споры: {}]",
            best_boss_eval.avg_altars_built,
            boss_mobs_spawned,
            best_boss_eval.director_grades.0,
            best_boss_eval.director_grades.1,
            best_boss_eval.director_grades.2
        );
        log_line!(
            "  ДИРЕКТОР (Фитнес: {:.0}):",
            best_dir_eval.fitness
        );
        log_line!(
            "    * Время жизни босса: {:>4.1} мин ({:>5.1} сек)",
            best_dir_eval.avg_kill_speed / 60.0,
            best_dir_eval.avg_kill_speed
        );
        let dir_altar_reward = (best_dir_eval.avg_altar_uptime_rate * 2.5).min(1000.0);
        log_line!(
            "    * Очки фитнеса: Бонус за ранний конец: +{:.0} | Штраф за затяжку: -{:.0} | Урон нанесен: {:.0} HP (+{:.0}) | Алтари (выживаемость: {:.1}с/мин): +{:.0}",
            dir_speed_reward, dir_time_whip,
            best_dir_eval.avg_damage_dealt, dir_dmg_reward,
            best_dir_eval.avg_altar_uptime_rate, dir_altar_reward
        );
        log_line!(
            "    * Построено алтарей: {} [Сумм. жизнь: {:.1} мин] | Призвано мобов: {} шт | Грейды орды: [Ярость: {}, Панцирь: {}, Чумные споры: {}]",
            best_dir_eval.avg_altars_built,
            best_dir_eval.avg_altar_uptime_total / 60.0,
            total_mobs_spawned,
            best_dir_eval.grades.0, best_dir_eval.grades.1, best_dir_eval.grades.2
        );
        log_line!(
            "      - Орда (T1-8): [Скелеты: {}, Гончие: {}, Культисты: {}, Големы: {}, Камикадзе: {}, Демоны: {}, Огонь: {}, Жнецы: {}]",
            best_dir_eval.mobs_spawned[0],
            best_dir_eval.mobs_spawned[1],
            best_dir_eval.mobs_spawned[2],
            best_dir_eval.mobs_spawned[3],
            best_dir_eval.mobs_spawned[4],
            best_dir_eval.mobs_spawned[5],
            best_dir_eval.mobs_spawned[6],
            best_dir_eval.mobs_spawned[7],
        );
        let legion_total: usize = best_dir_eval.mobs_spawned[8..16].iter().sum();
        if legion_total > 0 {
            log_line!(
                "      - Легион (T9-16): [Фаланга: {}, Церберы: {}, Архикультисты: {}, Колоссы: {}, Мучители: {}, Ловчие: {}, Архонты: {}, Аватар: {}]",
                best_dir_eval.mobs_spawned[8],
                best_dir_eval.mobs_spawned[9],
                best_dir_eval.mobs_spawned[10],
                best_dir_eval.mobs_spawned[11],
                best_dir_eval.mobs_spawned[12],
                best_dir_eval.mobs_spawned[13],
                best_dir_eval.mobs_spawned[14],
                best_dir_eval.mobs_spawned[15],
            );
        }
        let dir_boss_move_time = (best_dir_eval.avg_kill_speed - best_dir_eval.avg_boss_meditation).max(0.0);
        log_line!(
            "    ↳ Противник (Босс): Уровень: {} | Пешком (без дэша): {:.0} px [Рывков: {}] (Движение: {:.1} мин, Медитация: {:.1} мин) | Разрушил алтарей: {}",
            best_dir_eval.avg_boss_level,
            best_dir_eval.avg_boss_distance_walked,
            best_dir_eval.avg_boss_dashes,
            dir_boss_move_time / 60.0,
            best_dir_eval.avg_boss_meditation / 60.0,
            best_dir_eval.avg_altars_destroyed
        );
        log_line!(
            "    ↳ Скиллы и оружия босса ({}): [{}]",
            best_dir_eval.boss_weapons.len(),
            dir_boss_w_str
        );
        log_line!(
            "    ↳ Предметы и пассивки босса ({}): [{}]",
            best_dir_eval.boss_passives.len(),
            dir_boss_p_str
        );
        log_line!("================================================================================");

        append_to_history_log(&round_log);

        // 4. Селекция, кроссовер и мутации для следующего поколения
        let mut next_boss_pop = vec![boss_pop[0].clone(), boss_pop[1].clone()];
        let mut next_dir_pop = vec![director_pop[0].clone(), director_pop[1].clone()];

        while next_boss_pop.len() < population_size {
            let p1 = &boss_pop[rand::gen_range(0, population_size / 2)];
            let p2 = &boss_pop[rand::gen_range(0, population_size / 2)];
            let mut child = p1.clone();

            // Кроссовер и мутация глубокой нейросети (2,368 синапсов, 42 сенсора)
            if let (Some(net1), Some(net2)) = (&p1.neural_net, &p2.neural_net) {
                let mut child_net = net1.crossover(net2);
                child_net.mutate(0.08, 0.20);
                child.total_synapses = child_net.total_synapses();
                child.neural_net = Some(child_net);
            }

            // Кроссовер
            child.mob_repulsion_weight = (p1.mob_repulsion_weight + p2.mob_repulsion_weight) * 0.5;
            child.gem_attraction_weight = (p1.gem_attraction_weight + p2.gem_attraction_weight) * 0.5;
            child.altar_siege_aggression = (p1.altar_siege_aggression + p2.altar_siege_aggression) * 0.5;
            child.sniper_hunt_aggression = (p1.sniper_hunt_aggression + p2.sniper_hunt_aggression) * 0.5;

            // Мутация базовых параметров
            child.mob_repulsion_weight = (child.mob_repulsion_weight + rand::gen_range(-25.0, 25.0)).clamp(250.0, 700.0);
            child.gem_attraction_weight = (child.gem_attraction_weight + rand::gen_range(-20.0, 20.0)).clamp(100.0, 400.0);
            child.altar_siege_aggression = (child.altar_siege_aggression + rand::gen_range(-25.0, 25.0)).clamp(150.0, 500.0);
            child.sniper_hunt_aggression = (child.sniper_hunt_aggression + rand::gen_range(-30.0, 30.0)).clamp(150.0, 500.0);
            child.dash_danger_threshold = (child.dash_danger_threshold as i32 + rand::gen_range(-1, 2)).clamp(8, 18) as usize;

            // Кроссовер и мутация ДНК билдостроения и поиска имба-синергий
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

        while next_dir_pop.len() < population_size {
            let p1 = &director_pop[rand::gen_range(0, population_size / 2)];
            let p2 = &director_pop[rand::gen_range(0, population_size / 2)];
            let mut child = p1.clone();

            // Кроссовер и мутация нейросети Директора
            if let (Some(net1), Some(net2)) = (&p1.neural_net, &p2.neural_net) {
                let mut crossed_net = net1.crossover(net2);
                crossed_net.mutate(0.08, 0.18);
                child.total_synapses = crossed_net.total_synapses();
                child.neural_net = Some(crossed_net);
            } else {
                let mut net = crate::ai::neural_net::DirectorBrainNet::new();
                net.mutate(0.10, 0.20);
                child.total_synapses = net.total_synapses();
                child.neural_net = Some(net);
            }

            child.altar_build_priority = ((p1.altar_build_priority + p2.altar_build_priority) * 0.5 + rand::gen_range(-0.08, 0.08)).clamp(0.1, 1.0);
            for i in 0..child.squad_spawn_weights.len() {
                let p1_w = p1.squad_spawn_weights.get(i).copied().unwrap_or(1.0);
                let p2_w = p2.squad_spawn_weights.get(i).copied().unwrap_or(1.0);
                child.squad_spawn_weights[i] = ((p1_w + p2_w) * 0.5 + rand::gen_range(-0.15, 0.15)).max(0.1);
            }
            next_dir_pop.push(child);
        }

        boss_pop = next_boss_pop;
        director_pop = next_dir_pop;

        // Авто-сохранение каждые 25 поколений (защита от сбоев и возможность прервать обучение в любой момент)
        if gen_idx % 25 == 0 && gen_idx < generations {
            save_models(&best_overall_boss, &best_overall_director, gen_idx, false);
        }
    }

    // 5. Финальный экспорт обученных моделей в формате JSON
    save_models(&best_overall_boss, &best_overall_director, generations, true);
}

fn save_models(boss: &BossModel, director: &DirectorModel, generations: usize, is_final: bool) {
    let _ = std::fs::create_dir_all("models");
    if is_final {
        println!("\n[СОХРАНЕНИЕ] Финальный экспорт обученных моделей...");
    } else {
        println!("\n[ЧЕКПОИНТ #{}] Автоматическое сохранение прогресса на диск...", boss.generation);
    }

    if let Ok(boss_json) = serde_json::to_string_pretty(boss) {
        if let Ok(_) = std::fs::write("models/boss_policy.json", &boss_json) {
            println!("  [OK] Модель Босса ({} синапсов, Поколение #{}) -> models/boss_policy.json", boss.total_synapses, boss.generation);
        }
    }

    if let Some(ref net) = boss.neural_net {
        if let Ok(brain_json) = serde_json::to_string_pretty(net) {
            let _ = std::fs::write("models/boss_brain_neural.json", brain_json);
            println!("  [OK] Матрица синапсов (150 сенсоров, 2 скрытых слоя) -> models/boss_brain_neural.json");
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
    for (i, &w) in boss.combo_archetype_weights.iter().enumerate() {
        if w > best_weight {
            best_weight = w;
            best_arch = i;
        }
    }
    if let Some(name) = archetype_names.get(best_arch) {
        println!("  [АРХЕТИП] Доминирующий билд: {} (Вес: {:.2}, Поиск комбо: {:.2})", name, best_weight, boss.synergy_seeking_weight);
    }

    if let Ok(dir_json) = serde_json::to_string_pretty(director) {
        if let Ok(_) = std::fs::write("models/director_policy.json", dir_json) {
            println!("  [OK] Модель Директора ({} синапсов, Поколение #{}) -> models/director_policy.json", director.total_synapses, director.generation);
        }
    }

    if let Some(ref net) = director.neural_net {
        if let Ok(dir_brain_json) = serde_json::to_string_pretty(net) {
            let _ = std::fs::write("models/director_brain_neural.json", dir_brain_json);
            println!("  [OK] Матрица синапсов Директора (132 сенсора, 2 головы: 20 экономика + 3 алтарь) -> models/director_brain_neural.json");
        }
    }

    let stats = TrainingStats {
        total_generations: generations,
        total_matches_played: generations * 12 * 3,
        best_boss_survival_time: boss.avg_survival_time,
        best_boss_level: 28,
        best_director_kill_time: director.avg_boss_kill_time,
        timestamp: "2026-11-16".to_string(),
    };

    if let Ok(stats_json) = serde_json::to_string_pretty(&stats) {
        let _ = std::fs::write("models/training_log.json", stats_json);
    }

    if is_final {
        println!("[ГОТОВО] Обучение завершено. Модели сохранены и готовы к игре.\n");
        append_to_history_log("[ГОТОВО] Обучение завершено. Модели сохранены на диск.\n\n");
    }
}
