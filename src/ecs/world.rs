use macroquad::prelude::*;
use crate::core::rand;
use crate::core::spatial_grid::SpatialGrid;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction, Health, Resistances, StatusEffects};
use crate::ecs::components::inventory::{BossInventory, WeaponType};

#[derive(Clone, Debug)]
pub struct MobEntity {
    pub id: usize,
    pub pos: Vec2,
    pub vel: Vec2,
    pub speed: f32,
    pub health: Health,
    pub resistances: Resistances,
    pub status: StatusEffects,
    pub damage: f32,
    pub tier: u8,
    pub radius: f32,
    pub is_undead: bool,
    pub is_elite: bool,
    pub knockback_immune: bool,
    pub knockback_vel: Vec2,
    pub attack_timer: f32,
}

#[derive(Clone, Debug)]
pub struct ProjectileEntity {
    pub id: usize,
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub damage_type: DamageType,
    pub armor_penetration: f32,
    pub lifetime: f32,
    pub radius: f32,
    pub pierce: u8,
    pub faction: Faction,
    pub source_weapon: Option<WeaponType>,
    pub homing_strength: f32,
}

#[derive(Clone, Debug)]
pub struct SoulGemEntity {
    pub id: usize,
    pub pos: Vec2,
    pub xp_value: f32,
    pub is_magnet: bool, // Спец-предмет "Магнит", притягивающий весь опыт
}

#[derive(Clone, Debug)]
pub struct StructureEntity {
    pub id: usize,
    pub pos: Vec2,
    pub health: Health,
    pub resistances: Resistances,
    pub faction: Faction,
    pub radius: f32,
    pub structure_type: u8,
    pub pulse_timer: f32,
}

#[derive(Clone, Debug)]
pub struct AcidPoolEntity {
    pub pos: Vec2,
    pub radius: f32,
    pub lifetime: f32,
    pub dps: f32,
    pub corrosive: bool, // Снимает ли броню
    pub is_director_source: bool, // Принадлежит ли Директору (дамажит босса, а не мобов игрока!)
}

#[derive(Clone, Debug)]
pub struct FloatingTextEntity {
    pub pos: Vec2,
    pub vel: Vec2,
    pub text: String,
    pub color: Color,
    pub lifetime: f32,
    pub scale: f32,
}

#[derive(Clone, Debug)]
pub struct ParticleEntity {
    pub pos: Vec2,
    pub vel: Vec2,
    pub color: Color,
    pub radius: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

#[derive(Clone, Debug)]
pub struct DirectorState {
    pub id: u8,
    pub points: f32,
    pub income_rate: f32, // Очков каждые 2 секунды
    pub total_points_spent: f32,
    pub income_timer: f32,
    pub squad_cooldowns: [f32; 16],
    pub altar_cooldown: f32,
    pub altar_max_cooldown: f32,
    /// Время с момента последнего призыва отряда. Даёт Директору сенсор темпа.
    pub time_since_last_spawn: f32,
    pub total_mobs_spawned: usize,
    pub mob_page: u8,           // 0 = Тиры 1-8 (Базовая Орда), 1 = Тиры 9-16 (Высший Легион)
    pub grade_fury: u32,        // [F] Ярость: +12% урон, +6% скорость (u32: никогда не переполняется при 255+)
    pub grade_carapace: u32,    // [G] Панцирь: +2.5 броня, +5% физ. рез
    pub grade_pestilence: u32,  // [H] Чумные споры: отравление босса атаками мобов (ближними и дальними) + ядовитая миазма вокруг Алтарей
}

impl DirectorState {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            points: 30.0,
            income_rate: 8.0,
            total_points_spent: 0.0,
            income_timer: 0.0,
            squad_cooldowns: [0.0; 16],
            altar_cooldown: 0.0,
            altar_max_cooldown: 60.0, // 60 секунд (1 минута) кулдаун на постройку Алтаря (без искусственного лимита)
            time_since_last_spawn: 10.0,
            total_mobs_spawned: 0,
            mob_page: 0,
            grade_fury: 0,
            grade_carapace: 0,
            grade_pestilence: 0,
        }
    }

    pub fn spend_points(&mut self, cost: f32) -> bool {
        if self.points >= cost {
            self.points -= cost;
            self.total_points_spent += cost;
            // Spend to Grow: сбалансированная формула прироста дохода (+2% от стоимости покупки)
            self.income_rate += cost * 0.02;
            true
        } else {
            false
        }
    }
}

pub struct World {
    // Босс (Автономный соло-герой)
    pub boss_pos: Vec2,
    pub boss_vel: Vec2,
    pub boss_health: Health,
    pub boss_resistances: Resistances,
    pub boss_inventory: BossInventory,
    pub boss_dash_cooldown: f32,
    pub boss_invulnerable_timer: f32,
    pub boss_policy_think_timer: f32,
    pub boss_policy_has_cached_action: bool,
    pub boss_policy_move_dir: Vec2,
    pub boss_policy_move_mag: f32,
    pub boss_policy_dash: bool,
    pub boss_policy_dash_dir: Vec2,
    pub boss_policy_rest: bool,
    pub boss_still_time: f32, // Время стояния на месте (для медитации)
    pub boss_damage_taken_timer: f32, // Кулдаун после получения урона (3 сек блокирует медитацию)
    pub boss_mushroom_still_time: f32, // Время стояния для регенерации от Спорового Мицелия
    pub boss_poison_timer: f32, // Таймер отравления босса (от Чумных Спор Директора)
    pub boss_poison_dps: f32,   // Урон ядом в секунду
    pub game_time: f32, // Игровое время симуляции / сессии

    // Углы вращения для эволюций
    pub blade_storm_angle: f32,
    pub divine_shield_angle: f32,
    pub death_ray_active: Option<(Vec2, Vec2, f32)>, // start, end, lifetime

    // Пространственная сетка для O(1) поиска
    pub spatial_grid: SpatialGrid,

    // Мобы, снаряды, кристаллы (Data-Oriented плоские массивы)
    pub mobs: Vec<MobEntity>,
    pub projectiles: Vec<ProjectileEntity>,
    pub gems: Vec<SoulGemEntity>,
    pub structures: Vec<StructureEntity>,
    pub acid_pools: Vec<AcidPoolEntity>,
    pub floating_texts: Vec<FloatingTextEntity>,
    pub particles: Vec<ParticleEntity>,

    // Директор
    pub directors: Vec<DirectorState>,

    // Очередь урона
    pub damage_queue: Vec<DamagePacket>,

    pub next_entity_id: usize,
    pub total_damage_dealt_to_boss: f32,
    pub screen_shake: f32,
    pub boss_model: Option<crate::ai::models::BossModel>,
    pub audio_events: Vec<crate::audio::AudioEvent>,
    pub is_headless: bool, // Режим обучения без графики: отключает вложенный параллелизм
}

impl World {
    pub fn push_audio(&mut self, event: crate::audio::AudioEvent) {
        if self.audio_events.len() < 32 {
            self.audio_events.push(event);
        }
    }

    pub fn new() -> Self {
        Self {
            boss_pos: Vec2::new(crate::core::MAP_WIDTH * 0.5, crate::core::MAP_HEIGHT * 0.5),
            boss_vel: Vec2::ZERO,
            boss_health: Health::new(1200.0),
            boss_resistances: Resistances {
                flat_armor: 6.0,
                physical_res: 0.15,
                fire_res: 0.05,
                ice_res: 0.05,
                poison_res: 0.05,
                holy_res: 0.20,
            },
            boss_inventory: BossInventory::new(),
            boss_dash_cooldown: 0.0,
            boss_invulnerable_timer: 0.0,
            boss_policy_think_timer: 0.0,
            boss_policy_has_cached_action: false,
            boss_policy_move_dir: Vec2::ZERO,
            boss_policy_move_mag: 0.0,
            boss_policy_dash: false,
            boss_policy_dash_dir: Vec2::X,
            boss_policy_rest: false,
            boss_still_time: 0.0,
            boss_damage_taken_timer: 0.0,
            boss_mushroom_still_time: 0.0,
            boss_poison_timer: 0.0,
            boss_poison_dps: 0.0,
            game_time: 0.0,
            blade_storm_angle: 0.0,
            divine_shield_angle: 0.0,
            death_ray_active: None,
            spatial_grid: SpatialGrid::new(crate::core::MAP_WIDTH, crate::core::MAP_HEIGHT),
            mobs: Vec::with_capacity(3000),
            projectiles: Vec::with_capacity(1500),
            gems: Vec::with_capacity(3000),
            structures: Vec::new(),
            acid_pools: Vec::new(),
            floating_texts: Vec::with_capacity(500),
            particles: Vec::with_capacity(2000),
            directors: vec![DirectorState::new(1)],
            damage_queue: Vec::new(),
            next_entity_id: 1,
            total_damage_dealt_to_boss: 0.0,
            screen_shake: 0.0,
            // Модель загружается один раз в main.rs / симуляции, а не на каждом World::new().
            boss_model: None,
            audio_events: Vec::new(),
            is_headless: false,
        }
    }

    pub fn alloc_id(&mut self) -> usize {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    /// Перестраивает пространственную сетку по текущему списку мобов.
    /// Индексы в сетке валидны только до следующего удаления мобов.
    pub fn rebuild_spatial_grid(&mut self) {
        self.spatial_grid.clear();
        for (idx, mob) in self.mobs.iter().enumerate() {
            self.spatial_grid.insert(idx, mob.pos);
        }
    }

    pub fn add_floating_text(&mut self, pos: Vec2, text: String, color: Color, scale: f32) {
        if self.is_headless {
            return;
        }
        if self.floating_texts.len() < 300 {
            self.floating_texts.push(FloatingTextEntity {
                pos: pos + Vec2::new(rand::gen_range(-10.0, 10.0), rand::gen_range(-10.0, 5.0)),
                vel: Vec2::new(rand::gen_range(-15.0, 15.0), -45.0),
                text,
                color,
                lifetime: 0.8,
                scale,
            });
        }
    }

    pub fn spawn_particles(&mut self, pos: Vec2, count: usize, color: Color, speed_range: f32) {
        if self.is_headless {
            return;
        }
        if self.particles.len() + count < 1500 {
            for _ in 0..count {
                let angle = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                let speed = rand::gen_range(20.0, speed_range);
                let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
                let lifetime = rand::gen_range(0.2, 0.5);
                self.particles.push(ParticleEntity {
                    pos,
                    vel,
                    color,
                    radius: rand::gen_range(1.5, 3.5),
                    lifetime,
                    max_lifetime: lifetime,
                });
            }
        }
    }
}
