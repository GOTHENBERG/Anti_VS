use std::io::Write;
use std::time::{Duration, Instant};

use flate2::{Compression, write::ZlibEncoder};
use macroquad::prelude::*;

use crate::content::structures::{spawn_blood_altar, spawn_corner_obelisks, try_spawn_wall};
use crate::core::spatial_grid::{SpatialGrid, SpatialGridBackend};
use crate::core::time::{FIXED_SIM_DT, GameClock};
use crate::core::{MAP_HEIGHT, MAP_WIDTH};
use crate::ecs::components::combat::{DamageType, Faction, Health, Resistances, StatusEffects};
use crate::ecs::components::inventory::{PassiveSlot, PassiveType, WeaponSlot, WeaponType};
use crate::ecs::systems::{coop, movement};
use crate::ecs::world::{AcidPoolEntity, MobEntity, ProjectileEntity, SoulGemEntity, World};
use crate::network::session::{SNAPSHOT_CHUNK_PAYLOAD, decode_snapshot_payload};
use crate::network::snapshot::WorldSnapshot;
use crate::render::font::GameFonts;
use crate::render::renderer::render_world;
use crate::render::textures::GameTextures;
use crate::render::ui::render_ui;

#[derive(Clone, Copy)]
enum MobLayout {
    Scattered,
    DenseHorde,
}

impl MobLayout {
    fn label(self) -> &'static str {
        match self {
            Self::Scattered => "Scattered",
            Self::DenseHorde => "DenseHorde",
        }
    }
}

fn percentile_us(sorted_us: &[f64], pct: f64) -> f64 {
    if sorted_us.is_empty() {
        return 0.0;
    }
    let idx = (((sorted_us.len() - 1) as f64) * pct).round() as usize;
    sorted_us[idx.min(sorted_us.len() - 1)]
}

fn summarize_us(samples: &mut [Duration]) -> (f64, f64, f64) {
    let mut vals: Vec<f64> = samples
        .iter()
        .map(|d| d.as_secs_f64() * 1_000_000.0)
        .collect();
    vals.sort_by(|a, b| a.total_cmp(b));
    (
        percentile_us(&vals, 0.50),
        percentile_us(&vals, 0.95),
        percentile_us(&vals, 0.99),
    )
}

fn equip_benchmark_boss(inv: &mut crate::ecs::components::inventory::BossInventory) {
    inv.weapons = vec![
        WeaponSlot {
            kind: WeaponType::ArcaneScepter,
            level: 8,
            cooldown_timer: 0.0,
            base_cooldown: 0.35,
            damage: 32.0,
            is_evolved: false,
            limit_break_level: 0,
            bonus_projectiles: 2,
        },
        WeaponSlot {
            kind: WeaponType::BladeSquall,
            level: 8,
            cooldown_timer: 0.0,
            base_cooldown: 0.5,
            damage: 45.0,
            is_evolved: true,
            limit_break_level: 0,
            bonus_projectiles: 0,
        },
        WeaponSlot {
            kind: WeaponType::FireAura,
            level: 8,
            cooldown_timer: 0.0,
            base_cooldown: 0.5,
            damage: 40.0,
            is_evolved: false,
            limit_break_level: 0,
            bonus_projectiles: 0,
        },
        WeaponSlot {
            kind: WeaponType::ChainLightning,
            level: 8,
            cooldown_timer: 0.0,
            base_cooldown: 0.8,
            damage: 35.0,
            is_evolved: false,
            limit_break_level: 0,
            bonus_projectiles: 1,
        },
        WeaponSlot {
            kind: WeaponType::HolyHammer,
            level: 8,
            cooldown_timer: 0.0,
            base_cooldown: 0.7,
            damage: 38.0,
            is_evolved: false,
            limit_break_level: 0,
            bonus_projectiles: 0,
        },
        WeaponSlot {
            kind: WeaponType::NetherBlaze,
            level: 8,
            cooldown_timer: 0.0,
            base_cooldown: 0.6,
            damage: 55.0,
            is_evolved: true,
            limit_break_level: 0,
            bonus_projectiles: 0,
        },
    ];
    inv.passives = vec![
        PassiveSlot { kind: PassiveType::DivinityAura, level: 5 },
        PassiveSlot { kind: PassiveType::TeslaCoil, level: 5 },
        PassiveSlot { kind: PassiveType::UkuleleResonance, level: 3 },
        PassiveSlot { kind: PassiveType::CalamityMortar, level: 3 },
        PassiveSlot { kind: PassiveType::SoulHarvester, level: 3 },
        PassiveSlot { kind: PassiveType::GiantHeart, level: 5 },
    ];
}

fn spawn_bench_mobs(world: &mut World, count: usize, layout: MobLayout, centers: &[Vec2]) {
    world.mobs.reserve(count);
    for i in 0..count {
        let pos = match layout {
            MobLayout::Scattered => Vec2::new(
                100.0 + ((2137 * i) % 25_690) as f32,
                100.0 + ((1999 * i) % 17_070) as f32,
            ),
            MobLayout::DenseHorde => {
                if i % 5 == 0 {
                    Vec2::new(
                        100.0 + ((2137 * i) % 25_690) as f32,
                        100.0 + ((1999 * i) % 17_070) as f32,
                    )
                } else {
                    let c = centers[i % centers.len()];
                    let ring = 75.0 + ((i * 37) % 1100) as f32;
                    let ang = (i as f32) * 0.618_034 * std::f32::consts::TAU;
                    Vec2::new(
                        (c.x + ang.cos() * ring).clamp(40.0, MAP_WIDTH - 40.0),
                        (c.y + ang.sin() * ring).clamp(40.0, MAP_HEIGHT - 40.0),
                    )
                }
            }
        };
        let tier = ((i % 8) + 1) as u8;
        let id = 17 * i + 1;
        world.next_entity_id = world.next_entity_id.max(id + 1);
        let hp = if i % 40 == 0 { 20.0 } else { 25_000.0 };
        world.mobs.push(MobEntity {
            id,
            target_boss_id: None,
            pos,
            vel: Vec2::ZERO,
            speed: 65.0,
            health: Health::new(hp),
            resistances: Resistances::default(),
            status: StatusEffects::default(),
            damage: 0.0,
            tier,
            radius: 12.0,
            is_undead: tier == 1,
            is_elite: false,
            knockback_immune: false,
            knockback_vel: Vec2::ZERO,
            attack_timer: 0.0,
            is_hero: false,
            hero_enrage_timer: 0.0,
            hero_skill_timers: [0.0; 4],
            attack_count: 0,
            has_duplicated: false,
            owner_hero_id: None,
            last_hit_boss_id: None,
        });
    }
}

fn populate_bench_world(
    backend: SpatialGridBackend,
    mob_count: usize,
    boss_count: usize,
    layout: MobLayout,
    is_headless: bool,
    projectile_count: usize,
    gem_count: usize,
) -> World {
    crate::core::rand::seed(0xBADC_0FFE_E123_4567);
    let mut world = World::new();
    world.is_headless = is_headless;
    world.spatial_grid.set_backend(backend);

    let center = Vec2::new(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5);
    world.boss_pos = center;
    world.boss_health = Health::new(50_000.0);
    equip_benchmark_boss(&mut world.boss_inventory);

    let mut boss_centers = vec![center];
    for p_idx in 1..boss_count {
        let angle = (p_idx as f32) * std::f32::consts::TAU / (boss_count as f32);
        let pos = center + Vec2::new(angle.cos(), angle.sin()) * 420.0;
        let id = (p_idx + 1) as u8;
        world.add_coop_boss(id, pos, format!("Coop-{}", id), p_idx as u8);
        if let Some(player) = world.coop_bosses.last_mut() {
            player.boss_health = Health::new(50_000.0);
            equip_benchmark_boss(&mut player.boss_inventory);
        }
        boss_centers.push(pos);
    }

    // Алтари, обелиски и стены
    for k in 0..8 {
        let ang = (k as f32) * 0.785;
        spawn_blood_altar(
            &mut world,
            center + Vec2::new(ang.cos() * 480.0, ang.sin() * 420.0),
        );
    }
    spawn_corner_obelisks(&mut world);
    for w in 0..10 {
        let _ = try_spawn_wall(
            &mut world,
            Vec2::new(center.x - 300.0 + (w as f32) * 65.0, center.y + 320.0),
        );
    }

    spawn_bench_mobs(&mut world, mob_count, layout, &boss_centers);

    // Снаряды (смесь быстрых самонаводящихся босса и стрел Директора)
    for i in 0..projectile_count {
        let origin = boss_centers[i % boss_centers.len()];
        let angle = (i as f32) * 0.23;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let is_boss = i % 4 != 0;
        let pid = world.alloc_id();
        world.projectiles.push(ProjectileEntity {
            id: pid,
            pos: origin + dir * (35.0 + (i % 7) as f32 * 28.0),
            prev_pos: origin + dir * (35.0 + (i % 7) as f32 * 28.0),
            vel: dir * if is_boss { 520.0 } else { 320.0 },
            damage: 18.0,
            damage_type: if i % 3 == 0 { DamageType::Fire } else { DamageType::Holy },
            armor_penetration: 0.35,
            lifetime: 2.5,
            radius: 10.0,
            pierce: if is_boss { 3 } else { 1 },
            faction: if is_boss { Faction::Boss } else { Faction::Director },
            owner_boss_id: if is_boss { Some((i % boss_count + 1) as u8) } else { None },
            source_weapon: if is_boss { Some(WeaponType::ArcaneScepter) } else { None },
            homing_strength: if is_boss && i % 2 == 0 { 6.0 } else { 0.0 },
        });
    }

    for i in 0..16 {
        let ang = (i as f32) * 0.39;
        let origin = boss_centers[i % boss_centers.len()];
        world.acid_pools.push(AcidPoolEntity {
            pos: origin + Vec2::new(ang.cos(), ang.sin()) * 210.0,
            radius: 65.0,
            lifetime: 4.0,
            dps: 22.0,
            corrosive: i % 2 == 0,
            is_director_source: i % 5 == 0,
            owner_boss_id: Some((i % boss_count + 1) as u8),
        });
    }

    for i in 0..gem_count {
        let id = world.alloc_id();
        let ang = (i as f32) * 0.31;
        let r = 80.0 + ((i * 19) % 1400) as f32;
        world.gems.push(SoulGemEntity {
            id,
            pos: center + Vec2::new(ang.cos() * r, ang.sin() * r),
            xp_value: if i % 10 == 0 { 58.0 } else { 9.0 },
            is_magnet: false,
        });
    }

    world.rebuild_spatial_grid();
    world
}

/// Запускает полный набор бенчмарков Шага 4 в Release-сборке:
/// 1) Микротест перестроения и запросов (HashedLinked / DenseLinked / DenseCsr)
/// 2) Полный 20-Гц тик (0 / 1000 / 5000 / 20000 мобов, Scattered vs DenseHorde, 1 vs 4 боссов, headless seq + runtime par)
/// 3) Оценка реального сетевого трафика (from_world, JSON, zlib, чанки, MB/s, компактный бинарник)
pub fn run_step4_benchmarks() {
    println!("==========================================================================");
    println!(" STEP 4 BENCHMARK: Dense Linked vs Dense CSR (Full 20Hz Tick) & Network");
    println!("==========================================================================");

    run_spatial_microbenchmark();
    run_full_tick_matrix();
    run_network_traffic_benchmark();
}

fn run_spatial_microbenchmark() {
    println!("\n[1] SpatialGrid Microbenchmark (N=20,000 scattered mobs, 200 measured runs)");
    let n = 20_000;
    let mut positions = Vec::with_capacity(n);
    for i in 0..n {
        positions.push(Vec2::new(
            100.0 + ((2137 * i) % 25_690) as f32,
            100.0 + ((1999 * i) % 17_070) as f32,
        ));
    }
    let query_pts: Vec<Vec2> = (0..1024)
        .map(|i| {
            let cx = 5 + (191 * i) % 197;
            let cy = 5 + (103 * i) % 129;
            Vec2::new((cx as f32) * 128.0 - 192.0, (cy as f32) * 128.0 - 192.0)
        })
        .collect();
    let rays: Vec<(Vec2, Vec2)> = (0..128)
        .map(|i| {
            let a = query_pts[i * 8 % query_pts.len()];
            let ang = (i as f32) * 0.41;
            let len = if i % 16 == 0 { 1100.0 } else { 52.0 };
            (a, a + Vec2::new(ang.cos(), ang.sin()) * len)
        })
        .collect();

    for backend in [
        SpatialGridBackend::HashedLinked,
        SpatialGridBackend::DenseLinked,
        SpatialGridBackend::DenseCsr,
    ] {
        let mut grid = SpatialGrid::new_with_backend(MAP_WIDTH, MAP_HEIGHT, backend);
        let mut rebuild_samples = Vec::with_capacity(200);
        let mut cell20k_samples = Vec::with_capacity(120);
        let mut q3_samples = Vec::with_capacity(120);
        let mut q9_samples = Vec::with_capacity(120);
        let mut ray_samples = Vec::with_capacity(120);
        let mut q3_cands = 0usize;
        let mut q9_cands = 0usize;
        let mut ray_cands = 0usize;

        for _ in 0..20 {
            grid.rebuild_from_positions(&positions);
        }
        for _ in 0..200 {
            let t0 = Instant::now();
            grid.rebuild_from_positions(&positions);
            rebuild_samples.push(t0.elapsed());
        }

        let cell_coords: Vec<(usize, usize)> = positions
            .iter()
            .filter_map(|&p| grid.get_cell_coords(p))
            .collect();
        let mut scratch = Vec::with_capacity(256);

        for iter in 0..140 {
            let t0 = Instant::now();
            let mut checks_acc = 0usize;
            for (idx, &(cx, cy)) in cell_coords.iter().enumerate() {
                let mut seen = 0usize;
                grid.for_each_in_cell(cx, cy, |other| {
                    if other != idx {
                        seen += 1;
                    }
                    seen < 6
                });
                checks_acc += seen;
            }
            std::hint::black_box(checks_acc);
            if iter >= 20 {
                cell20k_samples.push(t0.elapsed());
            }

            let t1 = Instant::now();
            let mut sum3 = 0usize;
            for &pt in &query_pts {
                grid.query_nearby_into(pt, 128.0, &mut scratch);
                sum3 += scratch.len();
            }
            if iter >= 20 {
                q3_samples.push(t1.elapsed());
                q3_cands = sum3;
            }

            let t2 = Instant::now();
            let mut sum9 = 0usize;
            for &pt in &query_pts {
                grid.query_nearby_into(pt, 512.0, &mut scratch);
                sum9 += scratch.len();
            }
            if iter >= 20 {
                q9_samples.push(t2.elapsed());
                q9_cands = sum9;
            }

            let t3 = Instant::now();
            let mut rsum = 0usize;
            for &(a, b) in &rays {
                grid.query_segment_thick(a, b, 30.0, |id| rsum += id & 1 ^ 1);
            }
            std::hint::black_box(rsum);
            if iter >= 20 {
                ray_samples.push(t3.elapsed());
                ray_cands = rsum;
            }
        }

        let (rb_50, rb_95, rb_99) = summarize_us(&mut rebuild_samples);
        let (c20_50, c20_95, _) = summarize_us(&mut cell20k_samples);
        let (q3_50, q3_95, _) = summarize_us(&mut q3_samples);
        let (q9_50, q9_95, _) = summarize_us(&mut q9_samples);
        let (ry_50, ry_95, _) = summarize_us(&mut ray_samples);

        println!(
            "  {:<13} | rebuild {:6.1}/{:6.1}/{:6.1}us | 20k_cell {:6.1}/{:6.1}us | 1024x3x3 {:5.1}/{:5.1}us ({:5} cands) | 1024x9x9 {:5.1}/{:5.1}us ({:6} cands) | 128xSeg {:5.1}/{:5.1}us ({} cands)",
            format!("{:?}", backend),
            rb_50, rb_95, rb_99,
            c20_50, c20_95,
            q3_50, q3_95, q3_cands,
            q9_50, q9_95, q9_cands,
            ry_50, ry_95, ray_cands
        );
    }
}

struct FullTickPhases {
    effects_us: Vec<Duration>,
    movement_us: Vec<Duration>,
    combat_us: Vec<Duration>,
    total_us: Vec<Duration>,
}

fn measure_full_tick_scenario(
    backend: SpatialGridBackend,
    mob_count: usize,
    boss_count: usize,
    layout: MobLayout,
    is_headless: bool,
    warmup: usize,
    samples: usize,
) -> FullTickPhases {
    let mut world = populate_bench_world(backend, mob_count, boss_count, layout, is_headless, 160, 180);

    let mut effects_us = Vec::with_capacity(samples);
    let mut movement_us = Vec::with_capacity(samples);
    let mut combat_us = Vec::with_capacity(samples);
    let mut total_us = Vec::with_capacity(samples);

    let total_iters = warmup + samples;
    for iter in 0..total_iters {
        // Каждые 8 тиков обновляем сцену вне таймера, чтобы число мобов/снарядов
        // и плотность орды не дрейфовали в процессе замера.
        if iter > 0 && iter % 8 == 0 {
            world = populate_bench_world(backend, mob_count, boss_count, layout, is_headless, 160, 180);
        }
        // Не даём частицам/тексту накапливаться при is_headless = false.
        if !is_headless {
            world.particles.clear();
            world.floating_texts.clear();
        }

        let t_total0 = Instant::now();

        let t_eff0 = Instant::now();
        coop::update_all_boss_effects(&mut world, FIXED_SIM_DT);
        let d_eff = t_eff0.elapsed();

        let t_mov0 = Instant::now();
        movement::update_movement(&mut world, FIXED_SIM_DT);
        let d_mov = t_mov0.elapsed();

        let t_cmb0 = Instant::now();
        coop::update_coop_boss_environment(&mut world, FIXED_SIM_DT);
        coop::update_all_boss_combat(&mut world, FIXED_SIM_DT);
        coop::update_revives(&mut world, FIXED_SIM_DT);
        let d_cmb = t_cmb0.elapsed();

        let d_tot = t_total0.elapsed();

        if iter >= warmup {
            effects_us.push(d_eff);
            movement_us.push(d_mov);
            combat_us.push(d_cmb);
            total_us.push(d_tot);
        }
    }

    FullTickPhases {
        effects_us,
        movement_us,
        combat_us,
        total_us,
    }
}

fn run_full_tick_matrix() {
    println!("\n[2] FULL 20 Hz TICK BENCHMARK (Effects + Crowd + 2xRebuild + MicroSep + Swept Projectiles + Combat)");
    let cases: &[(usize, usize, MobLayout, bool, usize, usize)] = &[
        (0, 1, MobLayout::Scattered, true, 15, 60),
        (1_000, 1, MobLayout::Scattered, true, 15, 80),
        (1_000, 1, MobLayout::DenseHorde, true, 15, 80),
        (5_000, 1, MobLayout::Scattered, true, 15, 80),
        (5_000, 1, MobLayout::DenseHorde, true, 15, 80),
        (20_000, 1, MobLayout::Scattered, true, 12, 64),
        (20_000, 1, MobLayout::DenseHorde, true, 12, 64),
        (20_000, 4, MobLayout::DenseHorde, true, 12, 64),
        // Режим runtime Rayon (par_iter_mut) для 20k орды:
        (20_000, 1, MobLayout::DenseHorde, false, 12, 64),
        (20_000, 4, MobLayout::DenseHorde, false, 12, 64),
    ];

    for &(mobs, bosses, layout, headless, warmup, samples) in cases {
        let mode_str = if headless { "seq(headless)" } else { "par(runtime) " };
        println!(
            "\n  Scenario: mobs={:<5} bosses={} layout={:<10} {}",
            mobs,
            bosses,
            layout.label(),
            mode_str
        );
        for backend in [
            SpatialGridBackend::HashedLinked,
            SpatialGridBackend::DenseLinked,
            SpatialGridBackend::DenseCsr,
        ] {
            let mut res = measure_full_tick_scenario(backend, mobs, bosses, layout, headless, warmup, samples);
            let (eff_50, eff_95, _) = summarize_us(&mut res.effects_us);
            let (mov_50, mov_95, _) = summarize_us(&mut res.movement_us);
            let (cmb_50, cmb_95, _) = summarize_us(&mut res.combat_us);
            let (tot_50, tot_95, tot_99) = summarize_us(&mut res.total_us);
            println!(
                "    {:<12} | TOTAL p50={:7.1}us p95={:7.1}us p99={:7.1}us | eff={:6.1}/{:6.1} mov={:6.1}/{:6.1} cmb={:6.1}/{:6.1}",
                format!("{:?}", backend),
                tot_50, tot_95, tot_99,
                eff_50, eff_95,
                mov_50, mov_95,
                cmb_50, cmb_95
            );
        }
    }
}

fn estimate_compact_binary_snapshot_bytes(snap: &WorldSnapshot) -> usize {
    // Оценка плотного квантованного бинарного DTO без имен JSON-полей:
    // header(16) + per_boss(40 + skills*4) + director(24) + mob(16: u32 id, i16x2 pos, i16x2 vel, f32 hp/max или u16*2, status/flags/tier 4B)
    // + proj(18) + gem(10) + structure(24) + acid(14) + bh(20)
    let mut bytes = 16usize;
    for b in &snap.bosses {
        bytes += 48 + b.inventory.weapons.len() * 6 + b.inventory.passives.len() * 2;
    }
    if snap.director.is_some() {
        bytes += 24;
    }
    bytes += snap.mobs.len() * 16;
    bytes += snap.projectiles.len() * 18;
    bytes += snap.gems.len() * 10;
    bytes += snap.structures.len() * 24;
    bytes += snap.acid_pools.len() * 14;
    bytes += snap.black_holes.len() * 20;
    bytes
}

fn run_network_traffic_benchmark() {
    println!("\n[3] NETWORK REPLICATION & TRAFFIC PROFILE (Real active WorldSnapshot + JSON + zlib + UDP Chunks)");
    let scenarios: &[(usize, usize, usize, usize, &str)] = &[
        (0, 1, 0, 0, "Idle 0 mobs, 1 boss"),
        (100, 1, 24, 30, "Early 100 mobs, 1 boss, combat"),
        (600, 1, 160, 200, "Mid 600 mobs, 1 boss, combat"),
        (5_000, 4, 320, 350, "Late 5k mobs, 4 bosses, full cap"),
        (20_000, 4, 320, 350, "Stress 20k mobs, 4 bosses, full cap"),
        (20_000, 8, 320, 350, "Maxcoop 20k mobs, 8 bosses, full cap"),
    ];

    for &(mobs, bosses, projs, gems, label) in scenarios {
        let mut world = populate_bench_world(
            SpatialGridBackend::DenseLinked,
            mobs,
            bosses,
            MobLayout::DenseHorde,
            true,
            projs,
            gems,
        );
        // Для теста разнесённых vs кучных: один замер при плотной группе (Ring-BFS).
        let mut t_from = Vec::with_capacity(60);
        let mut t_json = Vec::with_capacity(60);
        let mut t_zlib = Vec::with_capacity(60);
        let mut t_decode = Vec::with_capacity(60);

        let mut json_bytes = 0usize;
        let mut zlib_bytes = 0usize;
        let mut compact_est = 0usize;
        let mut mob_cap = 0usize;

        for iter in 0..70 {
            let t0 = Instant::now();
            let snap = WorldSnapshot::from_world(&world, iter as u32 + 1);
            let d_from = t0.elapsed();

            let t1 = Instant::now();
            let json = serde_json::to_vec(&snap).unwrap();
            let d_json = t1.elapsed();

            let t2 = Instant::now();
            let mut enc = ZlibEncoder::new(Vec::with_capacity(json.len() / 8), Compression::fast());
            enc.write_all(&json).unwrap();
            let compressed = enc.finish().unwrap();
            let d_zlib = t2.elapsed();

            let t3 = Instant::now();
            let decoded = decode_snapshot_payload(&compressed).expect("valid snapshot");
            std::hint::black_box(&decoded);
            let d_dec = t3.elapsed();

            if iter >= 10 {
                t_from.push(d_from);
                t_json.push(d_json);
                t_zlib.push(d_zlib);
                t_decode.push(d_dec);
                json_bytes = json.len();
                zlib_bytes = compressed.len();
                compact_est = estimate_compact_binary_snapshot_bytes(&snap);
                mob_cap = snap.mobs.len();
            }
            let _ = &mut world;
        }

        let chunks = zlib_bytes.div_ceil(SNAPSHOT_CHUNK_PAYLOAD).max(1);
        let wire_per_snap = zlib_bytes + chunks * 12; // AVS1 12B заголовок
        let guest_kbps_10hz = (wire_per_snap * 10) as f64 / 1024.0;
        let guest_kbps_20hz = (wire_per_snap * 20) as f64 / 1024.0;
        let guests = (bosses.saturating_sub(1)).max(1);
        let host_mbps_10hz = (wire_per_snap * 10 * guests * 8) as f64 / 1_000_000.0;
        let host_mbps_20hz = (wire_per_snap * 20 * guests * 8) as f64 / 1_000_000.0;

        let (f50, f95, _) = summarize_us(&mut t_from);
        let (j50, _, _) = summarize_us(&mut t_json);
        let (z50, _, _) = summarize_us(&mut t_zlib);
        let (d50, _, _) = summarize_us(&mut t_decode);

        println!("  {:<38} | snap_mobs={:<3}", label, mob_cap);
        println!(
            "    sizes: raw_json={:>7}B -> zlib={:>6}B (ratio {:.2}x, chunks={:>2}, ~binary_est={}B)",
            json_bytes,
            zlib_bytes,
            json_bytes as f64 / zlib_bytes.max(1) as f64,
            chunks,
            compact_est
        );
        println!(
            "    cpu(p50): build={:5.1}us(p95={:5.1}) json={:5.1}us zlib={:5.1}us decode+parse={:6.1}us",
            f50, f95, j50, z50, d50
        );
        println!(
            "    net: 10Hz/guest={:6.1} KB/s (host x{}={:.2} Mbps) | 20Hz/guest={:6.1} KB/s (host x{}={:.2} Mbps) | P_loss1%={:.1}%",
            guest_kbps_10hz,
            guests,
            host_mbps_10hz,
            guest_kbps_20hz,
            guests,
            host_mbps_20hz,
            0.99f64.powi(chunks as i32) * 100.0
        );
    }
}

/// Профилировщик живого игрового кадра CPU + GPU (через реальное окно Macroquad) при нескольких уровнях зума и плотности.
pub async fn run_live_frame_profile(fonts: &GameFonts, textures: &GameTextures) {
    println!("==========================================================================");
    println!(" STEP 4 LIVE FRAME PROFILE (CPU Sim + RenderWorld + UI + GL/Present)");
    println!("==========================================================================");

    let cases: &[(usize, f32, MobLayout, &str)] = &[
        (0, 0.95, MobLayout::Scattered, "0 mobs, zoom 0.95"),
        (600, 0.95, MobLayout::DenseHorde, "600 horde, zoom 0.95"),
        (5_000, 0.95, MobLayout::DenseHorde, "5k horde, zoom 0.95"),
        (5_000, 0.45, MobLayout::DenseHorde, "5k horde, zoom 0.45 (Director)"),
        (20_000, 0.95, MobLayout::DenseHorde, "20k horde, zoom 0.95"),
        (20_000, 0.45, MobLayout::DenseHorde, "20k horde, zoom 0.45 (Director)"),
        (20_000, 0.25, MobLayout::DenseHorde, "20k horde, zoom 0.25 (Wide)"),
    ];

    for &(mob_count, cam_zoom, layout, label) in cases {
        let mut world = populate_bench_world(
            SpatialGridBackend::DenseLinked,
            mob_count,
            1,
            layout,
            false,
            160,
            240,
        );
        let mut clock = GameClock::new();
        let cam_pos = world.boss_pos;

        let mut sim_samples = Vec::new();
        let mut rworld_samples = Vec::with_capacity(40);
        let mut rui_samples = Vec::with_capacity(40);
        let mut present_samples = Vec::with_capacity(40);
        let mut total_samples = Vec::with_capacity(40);

        let mut last_vis_mobs = 0usize;
        let mut last_vis_projs = 0usize;
        let mut last_vis_gems = 0usize;
        let mut last_vis_tiles = 0usize;

        for frame in 0..50 {
            world.particles.clear();
            world.floating_texts.clear();

            let t_frame0 = Instant::now();

            // Симуляция на каждый 3-й кадр (имитация 20 Гц шага на ~60 Гц рендере)
            if frame % 3 == 0 {
                let t_s0 = Instant::now();
                clock.tick(FIXED_SIM_DT);
                coop::update_all_boss_effects(&mut world, FIXED_SIM_DT);
                movement::update_movement(&mut world, FIXED_SIM_DT);
                coop::update_coop_boss_environment(&mut world, FIXED_SIM_DT);
                coop::update_all_boss_combat(&mut world, FIXED_SIM_DT);
                coop::update_revives(&mut world, FIXED_SIM_DT);
                if frame >= 10 {
                    sim_samples.push(t_s0.elapsed());
                }
            }

            let screen_w = screen_width().max(1280.0);
            let screen_h = screen_height().max(720.0);
            let zoom_clamped = cam_zoom.max(0.15);
            let half_w = (screen_w / (2.0 * zoom_clamped)) + 90.0;
            let half_h = (screen_h / (2.0 * zoom_clamped)) + 90.0;

            let t_rw0 = Instant::now();
            let world_cam = Camera2D {
                target: cam_pos,
                zoom: Vec2::new(cam_zoom / (screen_w / 2.0), cam_zoom / (screen_h / 2.0)),
                ..Default::default()
            };
            set_camera(&world_cam);
            render_world(&world, fonts, textures, cam_pos, cam_zoom);
            let d_rw = t_rw0.elapsed();

            let t_ui0 = Instant::now();
            set_default_camera();
            let _ui = render_ui(
                &world,
                &clock,
                fonts,
                cam_pos,
                cam_zoom,
                textures,
                true,
                "Profile Track",
            );
            let d_ui = t_ui0.elapsed();

            let t_gl0 = Instant::now();
            next_frame().await;
            let d_gl = t_gl0.elapsed();
            let d_tot = t_frame0.elapsed();

            if frame == 49 {
                let in_rect = |p: Vec2, pad: f32| {
                    (p.x - cam_pos.x).abs() <= half_w + pad
                        && (p.y - cam_pos.y).abs() <= half_h + pad
                };
                last_vis_mobs = world.mobs.iter().filter(|m| in_rect(m.pos, m.radius + 15.0)).count();
                last_vis_projs = world.projectiles.iter().filter(|p| in_rect(p.pos, p.radius + 15.0)).count();
                last_vis_gems = world.gems.iter().filter(|g| in_rect(g.pos, 35.0)).count();
                let tiles_x = (((cam_pos.x + half_w).min(MAP_WIDTH) / 100.0).ceil()
                    - ((cam_pos.x - half_w).max(0.0) / 100.0).floor())
                    .max(0.0) as usize;
                let tiles_y = (((cam_pos.y + half_h).min(MAP_HEIGHT) / 100.0).ceil()
                    - ((cam_pos.y - half_h).max(0.0) / 100.0).floor())
                    .max(0.0) as usize;
                last_vis_tiles = tiles_x * tiles_y;
            }

            if frame >= 10 {
                rworld_samples.push(d_rw);
                rui_samples.push(d_ui);
                present_samples.push(d_gl);
                total_samples.push(d_tot);
            }
        }

        let (s50, s95, _) = summarize_us(&mut sim_samples);
        let (rw50, rw95, _) = summarize_us(&mut rworld_samples);
        let (ui50, ui95, _) = summarize_us(&mut rui_samples);
        let (gl50, gl95, _) = summarize_us(&mut present_samples);
        let (t50, t95, _) = summarize_us(&mut total_samples);

        println!("  {:<30} | visible: mobs={:<5} projs={:<3} gems={:<3} floor_tiles={:<5}", label, last_vis_mobs, last_vis_projs, last_vis_gems, last_vis_tiles);
        println!(
            "    ms(p50/p95): total={:.2}/{:.2}ms (~{:.0} FPS) | sim20Hz={:.2}/{:.2} | render_world={:.2}/{:.2} | render_ui={:.2}/{:.2} | next_frame(GL/vsync)={:.2}/{:.2}",
            t50 / 1000.0, t95 / 1000.0, 1_000_000.0 / t50.max(1.0),
            s50 / 1000.0, s95 / 1000.0,
            rw50 / 1000.0, rw95 / 1000.0,
            ui50 / 1000.0, ui95 / 1000.0,
            gl50 / 1000.0, gl95 / 1000.0
        );
    }
}
