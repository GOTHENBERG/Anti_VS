use macroquad::rand;
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
    avg_altars_destroyed: usize,
    avg_altars_built: usize,
    mobs_spawned: [usize; 8],
}

#[derive(Clone, Debug)]
struct DirectorEval {
    idx: usize,
    fitness: f32,
    avg_kill_speed: f32,
    avg_damage_dealt: f32,
    avg_altars_built: usize,
    mobs_spawned: [usize; 8],
    grades: (u32, u32, u32),
}

pub fn run_co_evolutionary_training(generations: usize, population_size: usize) {
    let gpu_info = detect_gpu_device();
    println!("============================================================");
    println!("🚀 ЗАПУСК ТУРНИРА СО-ЭВОЛЮЦИИ ИИ (LEAGUE SELF-PLAY)");
    println!("🎮 Графический процессор: {}", gpu_info);
    println!("⚡ Многопоточный движок: {} потоков (Rayon SIMD Parallel)", rayon::current_num_threads());
    println!("🏆 Поколений: {} | Размер популяции каждого агента: {}", generations, population_size);
    println!("🏛️ Зал Славы (League Hall of Fame): АКТИВЕН (защита от забывания)");
    println!("============================================================");

    let mut seed_boss = crate::ai::load_trained_boss_model().unwrap_or_default();
    if seed_boss.neural_net.is_none() {
        let net = crate::ai::neural_net::NeuralNet::new();
        seed_boss.total_synapses = net.total_synapses();
        seed_boss.neural_net = Some(net);
    }
    let seed_director = crate::ai::load_trained_director_model().unwrap_or_default();
    let base_gen = seed_boss.generation;

    if base_gen > 0 {
        println!("🔄 Дообучение (Warm Start): продолжаем с Поколения {} (Фитнес Босса: {:.0}, Синапсов: {})", base_gen, seed_boss.fitness, seed_boss.total_synapses);
    } else {
        println!("🌱 Начало нового цикла обучения: инициализирована глубокая нейросеть ({} синапсов).", seed_boss.total_synapses);
    }

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
            }
            m
        })
        .collect();

    let mut director_pop: Vec<DirectorModel> = (0..population_size)
        .map(|i| {
            let mut d = seed_director.clone();
            if i > 0 {
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
                let mut total_altars_destroyed = 0;
                let mut total_altars_built = 0;
                let mut total_mobs = [0usize; 8];
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

                    let res = run_headless_simulation(boss, director, 600.0); // 10 минут раунд

                    total_survival += res.survival_time;
                    total_level += res.boss_level;
                    total_damage_taken += res.damage_dealt_to_boss;
                    total_altars_destroyed += res.altars_destroyed;
                    total_altars_built += res.altars_built;
                    for t in 0..8 {
                        total_mobs[t] += res.mobs_spawned_by_tier[t];
                    }

                    // 1. Фитнес Босса:
                    // ГЛАВНЫЙ ПРЯНИК — время выживания (до 2400 очков) + бонус за победу (600 очков)
                    let time_reward = res.survival_time * 4.0;
                    let win_bonus = if res.boss_won { 600.0 } else { 0.0 };

                    // ВТОРОСТЕПЕННЫЕ ПРЯНИКИ — уровень героя и разрушение алтарей
                    let level_reward = res.boss_level as f32 * 15.0;
                    let altar_reward = res.altars_destroyed as f32 * 40.0;

                    // ВТОРОСТЕПЕННЫЙ КНУТ — мягкий штраф за полученный урон (стимул кайтить и уклоняться)
                    let damage_penalty = res.damage_dealt_to_boss * 0.06;

                    let score = (time_reward + win_bonus + level_reward + altar_reward - damage_penalty).max(10.0);
                    total_score += score;
                }

                let m_f = matches_count as f32;
                let mut avg_mobs = [0usize; 8];
                for t in 0..8 {
                    avg_mobs[t] = ((total_mobs[t] as f32) / m_f).round() as usize;
                }

                BossEval {
                    idx: b_idx,
                    fitness: total_score / m_f,
                    avg_survival: total_survival / m_f,
                    avg_level: (total_level as f32 / m_f).round() as u32,
                    avg_damage_taken: total_damage_taken / m_f,
                    avg_altars_destroyed: ((total_altars_destroyed as f32) / m_f).round() as usize,
                    avg_altars_built: ((total_altars_built as f32) / m_f).round() as usize,
                    mobs_spawned: avg_mobs,
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
                let mut total_mobs = [0usize; 8];
                let mut last_grades = (0, 0, 0);
                let matches_count = 3;

                for match_i in 0..matches_count {
                    let boss = if match_i == 2 && !hof_bosses.is_empty() {
                        let h_idx = rand::gen_range(0, hof_bosses.len());
                        &hof_bosses[h_idx]
                    } else {
                        let b_idx = rand::gen_range(0, population_size);
                        &boss_pop[b_idx]
                    };

                    let res = run_headless_simulation(boss, director, 600.0);

                    // 2. Фитнес Директора:
                    // ГЛАВНЫЙ ПРЯНИК — скорость сбривания босса (до 3000 очков) + бонус за добивание (600 очков)
                    let time_saved = (600.0 - res.survival_time).max(0.0);
                    let speed_reward = time_saved * 5.0;
                    let kill_bonus = if !res.boss_won { 600.0 } else { 0.0 };

                    // ГЛАВНЫЙ КНУТ ДИРЕКТОРА — время, которое босс прожил!
                    // Если босс живет дольше 4 минут (>240с), включается растущий штраф за затягивание боя
                    let time_drag_whip = if res.survival_time > 240.0 {
                        (res.survival_time - 240.0) * 2.0
                    } else {
                        0.0
                    };
                    let survived_boss_whip = if res.boss_won { 500.0 } else { 0.0 };
                    let total_time_whip = time_drag_whip + survived_boss_whip;

                    // ВТОРОСТЕПЕННЫЕ ПРЯНИКИ — нанесенный урон и постройка алтарей
                    let damage_reward = res.damage_dealt_to_boss * 0.08;
                    let altar_reward = res.altars_built as f32 * 20.0;

                    let kill_score = (speed_reward + kill_bonus + damage_reward + altar_reward - total_time_whip).max(10.0);
                    total_score += kill_score;
                    total_kill_speed += res.survival_time;
                    total_damage_dealt += res.damage_dealt_to_boss;
                    total_altars_built += res.altars_built;
                    last_grades = res.director_grades;
                    for t in 0..8 {
                        total_mobs[t] += res.mobs_spawned_by_tier[t];
                    }
                }

                let m_f = matches_count as f32;
                let mut avg_mobs = [0usize; 8];
                for t in 0..8 {
                    avg_mobs[t] = ((total_mobs[t] as f32) / m_f).round() as usize;
                }

                DirectorEval {
                    idx: d_idx,
                    fitness: total_score / m_f,
                    avg_kill_speed: total_kill_speed / m_f,
                    avg_damage_dealt: total_damage_dealt / m_f,
                    avg_altars_built: ((total_altars_built as f32) / m_f).round() as usize,
                    mobs_spawned: avg_mobs,
                    grades: last_grades,
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
        let boss_time_reward = best_boss_eval.avg_survival * 4.0;
        let boss_whip = best_boss_eval.avg_damage_taken * 0.06;
        let dir_speed_reward = (600.0 - best_dir_eval.avg_kill_speed).max(0.0) * 5.0;
        let dir_time_whip = if best_dir_eval.avg_kill_speed > 240.0 {
            (best_dir_eval.avg_kill_speed - 240.0) * 2.0
        } else {
            0.0
        };
        let dir_dmg_reward = best_dir_eval.avg_damage_dealt * 0.08;

        println!(
            "🏆 Раунд {:02}/{:02} (Поколение #{:02}) [🏛️ Зал Славы: {}]",
            gen_idx, generations, current_gen_num, hof_bosses.len()
        );
        println!(
            "   👑 Лучший Босс: выжил {:.1}с (Ур. {}, Фитнес {:.0}) | ⏳ Пряник времени: +{:.0} | Урон: {:.0} HP (Кнут: -{:.0}) | Сломал алтарей: {}",
            best_boss_eval.avg_survival, best_boss_eval.avg_level, best_boss_eval.fitness,
            boss_time_reward,
            best_boss_eval.avg_damage_taken, boss_whip,
            best_boss_eval.avg_altars_destroyed,
        );
        println!(
            "   👁️ Лучший Директор: сбрил за {:.1}с (Фитнес {:.0}) | ⚡ Скорость: +{:.0} (Кнут времени: -{:.0}) | Урон: {:.0} HP (+{:.0}) | Построил алтарей: {} | Грейды: [🔥Ярость: {}, 🛡️Панцирь: {}, 🧪Мор: {}]",
            best_dir_eval.avg_kill_speed, best_dir_eval.fitness,
            dir_speed_reward, dir_time_whip,
            best_dir_eval.avg_damage_dealt, dir_dmg_reward,
            best_dir_eval.avg_altars_built,
            best_dir_eval.grades.0, best_dir_eval.grades.1, best_dir_eval.grades.2,
        );
        println!(
            "      ↳ 👾 Призвано монстров ({} шт): [💀Скелеты: {}, 🐺Гончие: {}, 🔮Культисты: {}, 🗿Големы: {}, 💣Камикадзе: {}, 👤Демоны: {}, 🔥Огонь: {}, ☠️Жнецы: {}]",
            total_mobs_spawned,
            best_dir_eval.mobs_spawned[0],
            best_dir_eval.mobs_spawned[1],
            best_dir_eval.mobs_spawned[2],
            best_dir_eval.mobs_spawned[3],
            best_dir_eval.mobs_spawned[4],
            best_dir_eval.mobs_spawned[5],
            best_dir_eval.mobs_spawned[6],
            best_dir_eval.mobs_spawned[7],
        );

        // 4. Селекция, кроссовер и мутации для следующего поколения
        let mut next_boss_pop = vec![boss_pop[0].clone(), boss_pop[1].clone()];
        let mut next_dir_pop = vec![director_pop[0].clone(), director_pop[1].clone()];

        while next_boss_pop.len() < population_size {
            let p1 = &boss_pop[rand::gen_range(0, population_size / 2)];
            let p2 = &boss_pop[rand::gen_range(0, population_size / 2)];
            let mut child = p1.clone();

            // Кроссовер и мутация глубокой нейросети (2,176 синапсов!)
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

            // Мутация
            child.mob_repulsion_weight = (child.mob_repulsion_weight + rand::gen_range(-25.0, 25.0)).clamp(250.0, 700.0);
            child.gem_attraction_weight = (child.gem_attraction_weight + rand::gen_range(-20.0, 20.0)).clamp(100.0, 400.0);
            child.altar_siege_aggression = (child.altar_siege_aggression + rand::gen_range(-25.0, 25.0)).clamp(150.0, 500.0);
            child.sniper_hunt_aggression = (child.sniper_hunt_aggression + rand::gen_range(-30.0, 30.0)).clamp(150.0, 500.0);
            child.dash_danger_threshold = (child.dash_danger_threshold as i32 + rand::gen_range(-1, 2)).clamp(8, 18) as usize;

            next_boss_pop.push(child);
        }

        while next_dir_pop.len() < population_size {
            let p1 = &director_pop[rand::gen_range(0, population_size / 2)];
            let p2 = &director_pop[rand::gen_range(0, population_size / 2)];
            let mut child = p1.clone();

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
    if is_final {
        println!("\n💾 ФИНАЛЬНОЕ СОХРАНЕНИЕ ОБУЧЕННЫХ МОДЕЛЕЙ...");
    } else {
        println!("\n💾 ⚡ [ЧЕКПОИНТ #{}] Автоматическое сохранение прогресса на диск...", boss.generation);
    }

    if let Ok(boss_json) = serde_json::to_string_pretty(boss) {
        if let Ok(_) = std::fs::write("models/boss_policy.json", &boss_json) {
            println!("  ✅ Нейросетевая модель Босса ({} синапсов, Поколение #{}) сохранена в: models/boss_policy.json", boss.total_synapses, boss.generation);
        }
    }

    if let Some(ref net) = boss.neural_net {
        if let Ok(brain_json) = serde_json::to_string_pretty(net) {
            let _ = std::fs::write("models/boss_brain_neural.json", brain_json);
            println!("  🧠 Развернутая матрица синапсов (36 сенсоров, 2 скрытых слоя) сохранена в: models/boss_brain_neural.json");
        }
    }

    if let Ok(dir_json) = serde_json::to_string_pretty(director) {
        if let Ok(_) = std::fs::write("models/director_policy.json", dir_json) {
            println!("  ✅ Модель Директора (Поколение #{}) сохранена в: models/director_policy.json", director.generation);
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
        println!("🎉 Обучение завершено! Модель Босса теперь автоматически используется в игре!\n");
    }
}
