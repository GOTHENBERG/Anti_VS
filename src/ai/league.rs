use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ai::models::{BossModel, DirectorModel, MODEL_FORMAT_VERSION};
use crate::ai::neural_net::{BOSS_TRAINING_DECISION_INTERVAL, DirectorBrainNet, NeuralNet};
use crate::ai::sensors::TOTAL_SENSOR_SCHEMA_DIM;
use crate::ai::simulation::{MatchResult, run_headless_simulation_with_boss_count};
use crate::ai::trainer::director_utility;
use crate::core::rand;

pub const CONTROL_LEAGUE_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BossLeagueScore {
    pub fitness_mean: f32,
    pub win_rate: f32,
    pub avg_survival_frac: f32,
    pub avg_survival_secs: f32,
    pub avg_level: f32,
    pub avg_damage_taken: f32,
    pub avg_altars_destroyed: f32,
    pub avg_obelisks_deactivated: f32,
    pub matches: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorLeagueScore {
    pub fitness_mean: f32,
    pub win_rate: f32,
    pub avg_kill_time_secs: f32,
    pub avg_damage_dealt: f32,
    pub avg_altars_built: f32,
    pub avg_altar_uptime_secs: f32,
    pub matches: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ControlLeagueReport {
    pub league_version: u32,
    pub sensor_schema_dim: usize,
    pub model_format_version: u32,
    pub boss: BossLeagueScore,
    pub director: DirectorLeagueScore,
    pub digest: u64,
}

#[derive(Clone, Copy)]
struct LeagueMatchSpec {
    opponent_idx: usize,
    seed: u64,
    duration_secs: f32,
    boss_count: usize,
}

/// Фиксированный детерминированный набор соперников и матчей Контрольной Лиги.
/// Не зависит от изменяемых файлов в `models/`, поэтому позволяет сравнивать любые
/// поколения, версии правил и разные обучения на одинаковых условиях.
pub struct ControlLeagueRoster {
    pub boss_opponents: Vec<(&'static str, DirectorModel)>,
    pub director_opponents: Vec<(&'static str, BossModel)>,
    boss_matches: Vec<LeagueMatchSpec>,
    director_matches: Vec<LeagueMatchSpec>,
}

impl ControlLeagueRoster {
    pub fn canonical() -> Self {
        Self::build_with_caps(300.0, 1020.0)
    }

    pub fn fast_for_tests() -> Self {
        Self::build_with_caps(30.0, 30.0)
    }

    fn build_with_caps(short_cap: f32, long_cap: f32) -> Self {
        // Сохраняем текущий поток глобального RNG? Вызывающий код при необходимости
        // переинициализирует seed сам; здесь инициализация строго детерминирована.
        let mut dir_default = {
            rand::seed(0x4C45_4147_0001_0001);
            DirectorModel::default()
        };
        dir_default.version = "league-dir-balanced".to_string();

        let mut dir_swarm = {
            rand::seed(0x4C45_4147_0001_0002);
            let mut d = DirectorModel::default();
            if let Some(ref mut net) = d.neural_net {
                net.mutate(0.35, 0.28);
            }
            d.altar_build_priority = 0.45;
            d.soul_reserve_threshold = 20.0;
            d
        };
        dir_swarm.version = "league-dir-swarm".to_string();

        let mut dir_economy = {
            rand::seed(0x4C45_4147_0001_0003);
            let mut d = DirectorModel::default();
            if let Some(ref mut net) = d.neural_net {
                net.mutate(0.35, 0.25);
            }
            d.altar_build_priority = 1.45;
            d.soul_reserve_threshold = 80.0;
            d
        };
        dir_economy.version = "league-dir-altar-econ".to_string();

        let mut dir_pressure = {
            rand::seed(0x4C45_4147_0001_0004);
            let mut d = DirectorModel::default();
            let mut net = DirectorBrainNet::new();
            net.mutate(0.50, 0.32);
            d.neural_net = Some(net);
            d
        };
        dir_pressure.version = "league-dir-high-pressure".to_string();

        // 4 эталонных Босса: эвристика без нейросети + 3 детерминированных архетипа/сети
        let mut boss_heuristic = BossModel::default();
        boss_heuristic.neural_net = None;
        boss_heuristic.total_synapses = 0;
        boss_heuristic.version = "league-boss-heuristic".to_string();

        let boss_default = {
            rand::seed(0x4C45_4147_0002_0001);
            let mut b = BossModel::default();
            b.version = "league-boss-balanced-nn".to_string();
            b
        };

        let boss_pyro_bastion = {
            rand::seed(0x4C45_4147_0002_0002);
            let mut b = BossModel::default();
            if let Some(ref mut net) = b.neural_net {
                net.mutate(0.35, 0.26);
            }
            b.combo_archetype_weights = vec![3.2, 1.2, 3.0, 1.4, 2.2, 1.0];
            b.synergy_seeking_weight = 2.6;
            b.version = "league-boss-pyro-bastion".to_string();
            b
        };

        let boss_storm_hunter = {
            rand::seed(0x4C45_4147_0002_0003);
            let mut b = BossModel::default();
            let mut net = NeuralNet::new();
            net.mutate(0.40, 0.28);
            b.neural_net = Some(net);
            b.combo_archetype_weights = vec![1.0, 3.1, 1.5, 2.8, 1.6, 3.0];
            b.synergy_seeking_weight = 2.4;
            b.version = "league-boss-storm-hunter".to_string();
            b
        };

        let boss_opponents = vec![
            ("dir_balanced", dir_default),
            ("dir_swarm", dir_swarm),
            ("dir_altar_econ", dir_economy),
            ("dir_high_pressure", dir_pressure),
        ];
        let director_opponents = vec![
            ("boss_heuristic", boss_heuristic),
            ("boss_balanced_nn", boss_default),
            ("boss_pyro_bastion", boss_pyro_bastion),
            ("boss_storm_hunter", boss_storm_hunter),
        ];

        // Матрица матчей: 4 соперника * (2 коротких 300с + 1 полный 1020с) + 2 кооп-матча (4 босса, 300с) = 14 матчей/роль
        let mut boss_matches = Vec::with_capacity(14);
        let mut director_matches = Vec::with_capacity(14);
        let short_seeds = [0x1111_0001_u64, 0x2222_0002_u64];
        for opp in 0..4usize {
            for (s_i, &seed) in short_seeds.iter().enumerate() {
                boss_matches.push(LeagueMatchSpec {
                    opponent_idx: opp,
                    seed: seed ^ ((opp as u64 + 1) << 24) ^ (s_i as u64),
                    duration_secs: short_cap,
                    boss_count: 1,
                });
                director_matches.push(LeagueMatchSpec {
                    opponent_idx: opp,
                    seed: (seed ^ 0xFACE_0000) ^ ((opp as u64 + 1) << 24) ^ (s_i as u64),
                    duration_secs: short_cap,
                    boss_count: 1,
                });
            }
            boss_matches.push(LeagueMatchSpec {
                opponent_idx: opp,
                seed: 0x9999_0000_u64 ^ ((opp as u64 + 1) << 28),
                duration_secs: long_cap,
                boss_count: 1,
            });
            director_matches.push(LeagueMatchSpec {
                opponent_idx: opp,
                seed: 0xAAAA_0000_u64 ^ ((opp as u64 + 1) << 28),
                duration_secs: long_cap,
                boss_count: 1,
            });
        }
        // 2 контрольных кооп-матча (4 босса против фиксированных соперников)
        for coop_idx in 0..2usize {
            boss_matches.push(LeagueMatchSpec {
                opponent_idx: coop_idx * 2,
                seed: 0xC00B_0001_u64 + coop_idx as u64,
                duration_secs: short_cap,
                boss_count: 4,
            });
            director_matches.push(LeagueMatchSpec {
                opponent_idx: coop_idx * 2 + 1,
                seed: 0xC00D_0001_u64 + coop_idx as u64,
                duration_secs: short_cap,
                boss_count: 4,
            });
        }

        Self {
            boss_opponents,
            director_opponents,
            boss_matches,
            director_matches,
        }
    }

    pub fn evaluate(&self, boss_model: &BossModel, director_model: &DirectorModel) -> ControlLeagueReport {
        let boss_results: Vec<MatchResult> = self
            .boss_matches
            .par_iter()
            .map(|spec| {
                let (_, ref dir_opp) = self.boss_opponents[spec.opponent_idx];
                run_headless_simulation_with_boss_count(
                    boss_model,
                    dir_opp,
                    spec.seed,
                    spec.duration_secs,
                    BOSS_TRAINING_DECISION_INTERVAL,
                    spec.boss_count,
                )
            })
            .collect();

        let dir_results: Vec<MatchResult> = self
            .director_matches
            .par_iter()
            .map(|spec| {
                let (_, ref boss_opp) = self.director_opponents[spec.opponent_idx];
                run_headless_simulation_with_boss_count(
                    boss_opp,
                    director_model,
                    spec.seed,
                    spec.duration_secs,
                    BOSS_TRAINING_DECISION_INTERVAL,
                    spec.boss_count,
                )
            })
            .collect();

        let boss = summarize_boss_league(&boss_results);
        let director = summarize_director_league(&dir_results);
        let digest = compute_league_digest(&boss_results, &dir_results);

        ControlLeagueReport {
            league_version: CONTROL_LEAGUE_VERSION,
            sensor_schema_dim: TOTAL_SENSOR_SCHEMA_DIM,
            model_format_version: MODEL_FORMAT_VERSION,
            boss,
            director,
            digest,
        }
    }
}

fn summarize_boss_league(results: &[MatchResult]) -> BossLeagueScore {
    let n = results.len().max(1) as f32;
    let mut fitness_sum = 0.0f32;
    let mut wins = 0usize;
    let mut surv_frac_sum = 0.0f32;
    let mut surv_secs_sum = 0.0f32;
    let mut lvl_sum = 0u64;
    let mut dmg_sum = 0.0f32;
    let mut altars_sum = 0usize;
    let mut obelisks_sum = 0usize;

    for res in results {
        fitness_sum += -director_utility(res);
        if res.boss_won {
            wins += 1;
        }
        surv_frac_sum += (res.survival_time / res.match_cap.max(1.0)).clamp(0.0, 1.0);
        surv_secs_sum += res.survival_time;
        lvl_sum += res.boss_level as u64;
        dmg_sum += res.damage_dealt_to_boss;
        altars_sum += res.altars_destroyed;
        obelisks_sum += res.obelisks_deactivated;
    }

    BossLeagueScore {
        fitness_mean: fitness_sum / n,
        win_rate: wins as f32 / n,
        avg_survival_frac: surv_frac_sum / n,
        avg_survival_secs: surv_secs_sum / n,
        avg_level: lvl_sum as f32 / n,
        avg_damage_taken: dmg_sum / n,
        avg_altars_destroyed: altars_sum as f32 / n,
        avg_obelisks_deactivated: obelisks_sum as f32 / n,
        matches: results.len(),
    }
}

fn summarize_director_league(results: &[MatchResult]) -> DirectorLeagueScore {
    let n = results.len().max(1) as f32;
    let mut fitness_sum = 0.0f32;
    let mut wins = 0usize;
    let mut kill_time_sum = 0.0f32;
    let mut dmg_sum = 0.0f32;
    let mut altars_sum = 0usize;
    let mut uptime_sum = 0.0f32;

    for res in results {
        fitness_sum += director_utility(res);
        if !res.boss_won {
            wins += 1;
        }
        kill_time_sum += res.survival_time;
        dmg_sum += res.damage_dealt_to_boss;
        altars_sum += res.altars_built;
        uptime_sum += res.total_altar_uptime_seconds;
    }

    DirectorLeagueScore {
        fitness_mean: fitness_sum / n,
        win_rate: wins as f32 / n,
        avg_kill_time_secs: kill_time_sum / n,
        avg_damage_dealt: dmg_sum / n,
        avg_altars_built: altars_sum as f32 / n,
        avg_altar_uptime_secs: uptime_sum / n,
        matches: results.len(),
    }
}

fn compute_league_digest(boss_results: &[MatchResult], dir_results: &[MatchResult]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    };
    let norm = |v: f32| if v == 0.0 { 0.0f32.to_bits() } else { v.to_bits() };

    for res in boss_results.iter().chain(dir_results.iter()) {
        feed(&res.seed.to_le_bytes());
        feed(&norm(res.survival_time).to_le_bytes());
        feed(&[res.boss_won as u8]);
        feed(&norm(res.damage_dealt_to_boss).to_le_bytes());
        feed(&(res.boss_level as u32).to_le_bytes());
        feed(&(res.altars_built as u32).to_le_bytes());
        feed(&(res.altars_destroyed as u32).to_le_bytes());
        feed(&(res.obelisks_deactivated as u32).to_le_bytes());
    }
    hash
}

pub fn print_control_league_report(label: &str, report: &ControlLeagueReport) {
    println!("--------------------------------------------------------------------------");
    println!(
        " [CONTROL LEAGUE v{} | schema={} | format=v{} | digest={:#018x}] {}",
        report.league_version,
        report.sensor_schema_dim,
        report.model_format_version,
        report.digest,
        label
    );
    println!(
        "   BOSS     ({:>2} matches): fit={:+7.2} | win={:5.1}% | surv={:5.1}s ({:4.1}% cap) | lvl={:4.1} | dmg_taken={:6.0} | altars_destr={:.2} | obel={:.2}",
        report.boss.matches,
        report.boss.fitness_mean,
        report.boss.win_rate * 100.0,
        report.boss.avg_survival_secs,
        report.boss.avg_survival_frac * 100.0,
        report.boss.avg_level,
        report.boss.avg_damage_taken,
        report.boss.avg_altars_destroyed,
        report.boss.avg_obelisks_deactivated
    );
    println!(
        "   DIRECTOR ({:>2} matches): fit={:+7.2} | win={:5.1}% | end_time={:5.1}s | dmg_dealt={:6.0} | altars={:4.1} | uptime={:5.1}s",
        report.director.matches,
        report.director.fitness_mean,
        report.director.win_rate * 100.0,
        report.director.avg_kill_time_secs,
        report.director.avg_damage_dealt,
        report.director.avg_altars_built,
        report.director.avg_altar_uptime_secs
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_league_deterministic_and_finite() {
        let roster = ControlLeagueRoster::fast_for_tests();
        rand::seed(0x5555_1234);
        let boss = BossModel::default();
        let dir = DirectorModel::default();

        let rep_a = roster.evaluate(&boss, &dir);
        let rep_b = roster.evaluate(&boss, &dir);

        assert_eq!(rep_a.digest, rep_b.digest, "Control league digest must be reproducible");
        assert_eq!(rep_a.sensor_schema_dim, 437);
        assert_eq!(rep_a.boss.matches, 14);
        assert_eq!(rep_a.director.matches, 14);
        assert!(rep_a.boss.fitness_mean.is_finite());
        assert!(rep_a.director.fitness_mean.is_finite());
    }
}
