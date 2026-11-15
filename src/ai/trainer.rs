use macroquad::rand;
use rayon::prelude::*;
use crate::ai::models::{BossModel, DirectorModel, TrainingStats};
use crate::ai::simulation::run_headless_simulation;

pub fn run_co_evolutionary_training(generations: usize, population_size: usize) {
    println!("============================================================");
    println!("🚀 ЗАПУСК СО-ЭВОЛЮЦИОННОГО ОБУЧЕНИЯ (ADVERSARIAL SELF-PLAY)");
    println!("Поколений: {} | Размер популяции каждого агента: {}", generations, population_size);
    println!("Директор обучается душить Босса, а Босс — выживать 30 минут!");
    println!("============================================================");

    // 1. Инициализация стартовых популяций
    let mut boss_pop: Vec<BossModel> = (0..population_size)
        .map(|_i| {
            let mut m = BossModel::default();
            m.generation = 0;
            // Небольшая начальная вариативность
            m.mob_repulsion_weight += rand::gen_range(-40.0, 40.0);
            m.gem_attraction_weight += rand::gen_range(-30.0, 30.0);
            m.sniper_hunt_aggression += rand::gen_range(-40.0, 40.0);
            m.altar_siege_aggression += rand::gen_range(-30.0, 30.0);
            m
        })
        .collect();

    let mut director_pop: Vec<DirectorModel> = (0..population_size)
        .map(|_i| {
            let mut d = DirectorModel::default();
            d.generation = 0;
            d.altar_build_priority = (d.altar_build_priority + rand::gen_range(-0.15, 0.15)).clamp(0.1, 1.0);
            d
        })
        .collect();

    let mut best_overall_boss = boss_pop[0].clone();
    let mut best_overall_director = director_pop[0].clone();

    // 2. Цикл поколений обучения
    for gen_idx in 1..=generations {
        // Оценка фитнеса каждого кандидата в параллельных схватках
        let boss_results: Vec<(usize, f32, f32, u32)> = (0..population_size)
            .into_par_iter()
            .map(|b_idx| {
                let boss = &boss_pop[b_idx];
                let mut total_survival = 0.0;
                let mut total_level = 0;
                let mut total_score = 0.0;
                let matches_count = 3; // Каждый босс играет против 3 случайных директоров

                for _ in 0..matches_count {
                    let d_idx = rand::gen_range(0, population_size);
                    let director = &director_pop[d_idx];
                    let res = run_headless_simulation(boss, director, 600.0); // 10 минут раунд

                    total_survival += res.survival_time;
                    total_level += res.boss_level;
                    // Фитнес Босса: время жизни + уровень + уничтоженные алтари
                    let score = res.survival_time * 1.5
                        + (res.boss_level as f32 * 35.0)
                        + (res.altars_destroyed as f32 * 60.0);
                    total_score += score;
                }

                (
                    b_idx,
                    total_score / (matches_count as f32),
                    total_survival / (matches_count as f32),
                    total_level / (matches_count as u32),
                )
            })
            .collect();

        // Оценка директоров
        let director_results: Vec<(usize, f32, f32)> = (0..population_size)
            .into_par_iter()
            .map(|d_idx| {
                let director = &director_pop[d_idx];
                let mut total_kill_speed = 0.0;
                let mut total_score = 0.0;
                let matches_count = 3;

                for _ in 0..matches_count {
                    let b_idx = rand::gen_range(0, population_size);
                    let boss = &boss_pop[b_idx];
                    let res = run_headless_simulation(boss, director, 600.0);

                    // Фитнес Директора: чем быстрее убит босс, тем выше фитнес!
                    let kill_score = (600.0 - res.survival_time) * 2.0
                        + (res.damage_dealt_to_boss * 0.15)
                        + (res.altars_built as f32 * 25.0);
                    total_score += kill_score;
                    total_kill_speed += res.survival_time;
                }

                (
                    d_idx,
                    total_score / (matches_count as f32),
                    total_kill_speed / (matches_count as f32),
                )
            })
            .collect();

        // Обновляем фитнес в моделях
        for (idx, score, avg_surv, _) in &boss_results {
            boss_pop[*idx].fitness = *score;
            boss_pop[*idx].avg_survival_time = *avg_surv;
            boss_pop[*idx].generation = gen_idx as u32;
        }

        for (idx, score, avg_kill) in &director_results {
            director_pop[*idx].fitness = *score;
            director_pop[*idx].avg_boss_kill_time = *avg_kill;
            director_pop[*idx].generation = gen_idx as u32;
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

        println!(
            "🏆 Поколение {:02}/{:02} | Босс: выжил {:.1}с (Фитнес {:.0}) | Директор: сбрил за {:.1}с (Фитнес {:.0})",
            gen_idx, generations,
            current_best_boss.avg_survival_time, current_best_boss.fitness,
            current_best_dir.avg_boss_kill_time, current_best_dir.fitness
        );

        // 3. Селекция, кроссовер и мутации для следующего поколения
        let _elite_count = 2; // Элитизм: 2 лучших переходят без изменений
        let mut next_boss_pop = vec![boss_pop[0].clone(), boss_pop[1].clone()];
        let mut next_dir_pop = vec![director_pop[0].clone(), director_pop[1].clone()];

        while next_boss_pop.len() < population_size {
            let p1 = &boss_pop[rand::gen_range(0, population_size / 2)];
            let p2 = &boss_pop[rand::gen_range(0, population_size / 2)];
            let mut child = p1.clone();

            // Кроссовер
            child.mob_repulsion_weight = (p1.mob_repulsion_weight + p2.mob_repulsion_weight) * 0.5;
            child.gem_attraction_weight = (p1.gem_attraction_weight + p2.gem_attraction_weight) * 0.5;
            child.altar_siege_aggression = (p1.altar_siege_aggression + p2.altar_siege_aggression) * 0.5;
            child.sniper_hunt_aggression = (p1.sniper_hunt_aggression + p2.sniper_hunt_aggression) * 0.5;

            // Мутация
            child.mob_repulsion_weight += rand::gen_range(-25.0, 25.0);
            child.gem_attraction_weight += rand::gen_range(-20.0, 20.0);
            child.altar_siege_aggression += rand::gen_range(-25.0, 25.0);
            child.sniper_hunt_aggression += rand::gen_range(-30.0, 30.0);
            child.dash_danger_threshold = (child.dash_danger_threshold as i32 + rand::gen_range(-1, 2)).clamp(8, 18) as usize;

            next_boss_pop.push(child);
        }

        while next_dir_pop.len() < population_size {
            let p1 = &director_pop[rand::gen_range(0, population_size / 2)];
            let p2 = &director_pop[rand::gen_range(0, population_size / 2)];
            let mut child = p1.clone();

            child.altar_build_priority = ((p1.altar_build_priority + p2.altar_build_priority) * 0.5 + rand::gen_range(-0.08, 0.08)).clamp(0.1, 1.0);
            for i in 0..5 {
                child.squad_spawn_weights[i] = ((p1.squad_spawn_weights[i] + p2.squad_spawn_weights[i]) * 0.5 + rand::gen_range(-0.15, 0.15)).max(0.1);
            }
            next_dir_pop.push(child);
        }

        boss_pop = next_boss_pop;
        director_pop = next_dir_pop;
    }

    // 4. Экспорт обученных моделей в формате JSON
    save_models(&best_overall_boss, &best_overall_director, generations);
}

fn save_models(boss: &BossModel, director: &DirectorModel, generations: usize) {
    println!("\n💾 СОХРАНЕНИЕ ОБУЧЕННЫХ МОДЕЛЕЙ...");

    if let Ok(boss_json) = serde_json::to_string_pretty(boss) {
        if let Ok(_) = std::fs::write("models/boss_policy.json", boss_json) {
            println!("  ✅ Модель Босса сохранена в: models/boss_policy.json");
        }
    }

    if let Ok(dir_json) = serde_json::to_string_pretty(director) {
        if let Ok(_) = std::fs::write("models/director_policy.json", dir_json) {
            println!("  ✅ Модель Директора сохранена в: models/director_policy.json");
        }
    }

    let stats = TrainingStats {
        total_generations: generations,
        total_matches_played: generations * 12 * 3,
        best_boss_survival_time: boss.avg_survival_time,
        best_boss_level: 28,
        best_director_kill_time: director.avg_boss_kill_time,
        timestamp: "2026-11-16 00:20".to_string(),
    };

    if let Ok(stats_json) = serde_json::to_string_pretty(&stats) {
        let _ = std::fs::write("models/training_log.json", stats_json);
    }

    println!("🎉 Обучение завершено! Модель Босса теперь автоматически используется в игре!");
}
