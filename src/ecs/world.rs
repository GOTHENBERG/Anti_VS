use macroquad::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::core::spatial_grid::SpatialGrid;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction, Health, Resistances, StatusEffects};
use crate::ecs::components::inventory::{BossInventory, WeaponType};

#[derive(Clone, Debug)]
pub struct MobEntity {
    pub id: usize,
    /// Boss player this Director squad was spawned to pursue; altar mobs use None.
    pub target_boss_id: Option<u8>,
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
    pub is_hero: bool,
    pub hero_enrage_timer: f32,
    pub hero_skill_timers: [f32; 4],
    pub attack_count: u32,
    pub has_duplicated: bool,
    pub owner_hero_id: Option<usize>,
    pub last_hit_boss_id: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct BlackHoleEntity {
    pub id: usize,
    pub owner_id: usize,
    pub center_pos: Vec2,
    pub radius: f32,
    pub orbit_angle: f32,
    pub orbit_dist: f32,
    pub orbit_speed: f32,
    pub duration: f32,
    pub is_active: bool,
    pub spin_angle: f32,
}

impl MobEntity {
    #[inline]
    pub fn is_ranged(&self) -> bool {
        crate::content::mobs::is_ranged_tier(self.tier)
    }

    #[cfg(test)]
    pub fn new_test(id: usize, pos: Vec2, tier: u8) -> Self {
        Self {
            id,
            target_boss_id: None,
            pos,
            vel: Vec2::ZERO,
            speed: 100.0,
            health: Health::new(100.0),
            resistances: Resistances::default(),
            status: StatusEffects::default(),
            damage: 10.0,
            tier,
            radius: 12.0,
            is_undead: false,
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
        }
    }
}

pub const MAX_DIRECTOR_GRADE: u32 = 30;

#[derive(Clone, Debug)]
pub struct ProjectileEntity {
    pub id: usize,
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub damage_type: DamageType,
    pub armor_penetration: f32,
    pub lifetime: f32,
    pub radius: f32,
    pub pierce: u8,
    pub faction: Faction,
    pub owner_boss_id: Option<u8>,
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
    pub spawn_timer: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub is_active: bool,
    pub state_timer: f32,
}

#[derive(Clone, Debug)]
pub struct AcidPoolEntity {
    pub pos: Vec2,
    pub radius: f32,
    pub lifetime: f32,
    pub dps: f32,
    pub corrosive: bool, // Снимает ли броню
    pub is_director_source: bool, // Принадлежит ли Директору (дамажит босса, а не мобов игрока!)
    pub owner_boss_id: Option<u8>,
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
pub struct MoveClickEffect {
    pub pos: Vec2,
    pub timer: f32,
    pub duration: f32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DirectorState {
    pub id: u8,
    pub points: f32,
    pub income_rate: f32, // Очков каждые 2 секунды
    pub total_points_spent: f32,
    pub income_timer: f32,
    pub squad_cooldowns: [f32; 16],
    pub altar_cooldown: f32,
    pub altar_max_cooldown: f32,
    /// Independent placement cooldowns for tier 2..=16 altars (tier 1 keeps legacy fields).
    pub altar_tier_cooldowns: [f32; 15],
    pub wall_cooldown: f32,
    pub wall_max_cooldown: f32,
    /// Время с момента последнего призыва отряда. Даёт Директору сенсор темпа.
    pub time_since_last_spawn: f32,
    pub total_mobs_spawned: usize,
    pub mob_page: u8,           // 0 = Тиры 1-8 (Базовая Орда), 1 = Тиры 9-16 (Высший Легион), 2 = Герои
    pub grade_fury: u32,        // [F] Ярость: +12% урон, +6% скорость (u32: никогда не переполняется при 255+)
    pub grade_carapace: u32,    // [G] Панцирь: +2.5 броня, +5% физ. рез
    pub grade_pestilence: u32,  // [H] Чумные споры: отравление босса атаками мобов (ближними и дальними) + ядовитая миазма вокруг Алтарей
    #[serde(default)]
    pub hero_cooldown: f32,
    #[serde(default = "default_hero_max_cd")]
    pub hero_max_cooldown: f32,
}

fn default_hero_max_cd() -> f32 {
    60.0
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
            altar_max_cooldown: crate::content::structures::altar_cooldown(1),
            altar_tier_cooldowns: [0.0; 15],
            wall_cooldown: 0.0,
            wall_max_cooldown: 2.0, // 2 секунды кулдаун на постройку Стены-Баррикады
            time_since_last_spawn: 10.0,
            total_mobs_spawned: 0,
            mob_page: 0,
            grade_fury: 0,
            grade_carapace: 0,
            grade_pestilence: 0,
            hero_cooldown: 0.0,
            hero_max_cooldown: 60.0,
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

    pub fn altar_cooldown_for_tier(&self, tier: u8) -> f32 {
        if tier <= 1 {
            self.altar_cooldown
        } else {
            self.altar_tier_cooldowns[(tier.clamp(2, 16) - 2) as usize]
        }
    }

    pub fn set_altar_cooldown_for_tier(&mut self, tier: u8, seconds: f32) {
        if tier <= 1 {
            self.altar_cooldown = seconds;
            self.altar_max_cooldown = seconds;
        } else {
            self.altar_tier_cooldowns[(tier.clamp(2, 16) - 2) as usize] = seconds;
        }
    }
}

/// Runtime state owned by an additional Boss player in a co-op session.
#[derive(Clone, Debug)]
pub struct BossPlayerState {
    pub id: u8,
    pub boss_pos: Vec2,
    pub boss_vel: Vec2,
    pub boss_facing: Vec2,
    pub boss_name: String,
    pub boss_skin: u8,
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
    pub boss_still_time: f32,
    pub boss_damage_taken_timer: f32,
    pub boss_mushroom_still_time: f32,
    pub boss_poison_timer: f32,
    pub boss_poison_dps: f32,
    pub blade_squall_angle: f32,
    pub light_bastion_angle: f32,
    pub death_ray_active: Option<(Vec2, Vec2, f32)>,
    pub revive_progress: f32,
    pub revive_target_id: Option<u8>,
    pub pending_level_ups: usize,
    pub pending_relic_choices: usize,
    pub human_controlled: bool,
    pub level_up_options: Option<Vec<crate::ecs::components::inventory::PerkChoice>>,
    pub input_move_dir: Vec2,
    pub input_aim_dir: Vec2,
    pub input_dash_pressed: bool,
}

impl BossPlayerState {
    pub fn new(id: u8, pos: Vec2, name: impl Into<String>, skin: u8) -> Self {
        Self {
            id,
            boss_pos: pos,
            boss_vel: Vec2::ZERO,
            boss_facing: Vec2::X,
            boss_name: name.into(),
            boss_skin: skin,
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
            blade_squall_angle: 0.0,
            light_bastion_angle: 0.0,
            death_ray_active: None,
            revive_progress: 0.0,
            revive_target_id: None,
            pending_level_ups: 0,
            pending_relic_choices: 0,
            human_controlled: false,
            level_up_options: None,
            input_move_dir: Vec2::ZERO,
            input_aim_dir: Vec2::X,
            input_dash_pressed: false,
        }
    }
}

pub struct World {
    // Босс (Автономный соло-герой)
    pub boss_pos: Vec2,
    pub boss_vel: Vec2,
    pub boss_facing: Vec2,
    pub boss_name: String,
    pub boss_skin: u8,
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
    pub boss_still_time: f32,
    pub boss_damage_taken_timer: f32,
    pub boss_mushroom_still_time: f32,
    pub boss_poison_timer: f32,
    pub boss_poison_dps: f32,
    pub boss_revive_progress: f32,
    pub boss_human_controlled: bool,
    pub boss_input_move_dir: Vec2,
    pub boss_input_aim_dir: Vec2,
    pub boss_input_dash_pressed: bool,
    pub boss_pending_level_ups: usize,
    pub boss_pending_relic_choices: usize,
    pub boss_level_up_options: Option<Vec<crate::ecs::components::inventory::PerkChoice>>,
    /// ID of the Boss state temporarily loaded into the legacy `boss_*` fields.
    pub active_boss_id: u8,
    pub blade_squall_angle: f32,
    pub light_bastion_angle: f32,
    pub death_ray_active: Option<(Vec2, Vec2, f32)>,
    pub coop_bosses: Vec<BossPlayerState>,
    pub obelisk_contributors: HashMap<usize, HashSet<u8>>,
    pub structure_contributors: HashMap<usize, HashSet<u8>>,
    pub game_time: f32, // Игровое время симуляции / сессии
    pub match_duration_secs: f32,

    // Пространственная сетка для O(1) поиска
    pub spatial_grid: SpatialGrid,

    // Сетка сплошной среды для гидродинамики толпы (Continuum Crowds)
    pub crowd_grid: crate::core::crowd::CrowdDensityGrid,
    pub mob_positions_scratch: Vec<Vec2>,

    // Векторное поле обтекания препятствий (Flow Field)
    pub flow_field: crate::ai::flow_field::FlowField,

    // Мобы, снаряды, кристаллы (Data-Oriented плоские массивы)
    pub mobs: Vec<MobEntity>,
    pub projectiles: Vec<ProjectileEntity>,
    pub gems: Vec<SoulGemEntity>,
    pub structures: Vec<StructureEntity>,
    pub acid_pools: Vec<AcidPoolEntity>,
    pub black_holes: Vec<BlackHoleEntity>,
    pub floating_texts: Vec<FloatingTextEntity>,
    pub particles: Vec<ParticleEntity>,
    pub lightning_arcs: Vec<(Vec2, Vec2, f32)>, // (start, end, timer)
    pub boss_waypoint_queue: VecDeque<Vec2>,
    pub move_click_effects: Vec<MoveClickEffect>,

    // Директор
    pub directors: Vec<DirectorState>,

    // Очередь урона
    pub damage_queue: Vec<DamagePacket>,

    pub next_entity_id: usize,
    pub total_damage_dealt_to_boss: f32,
    pub boss_altars_destroyed_total: usize,
    pub boss_obelisks_deactivated_total: usize,
    pub screen_shake: f32,
    pub soul_link_tension: f32,
    pub obelisk_cycle_timer: f32,
    pub boss_model: Option<crate::ai::models::BossModel>,
    pub audio_events: Vec<crate::audio::AudioEvent>,
    pub is_headless: bool, // Режим обучения без графики: отключает вложенный параллелизм
    pub is_network_paused: bool, // Глобальная пауза сетевого матча
    pub pause_initiator: Option<String>, // Имя/роль игрока, поставившего матч на паузу
}

impl World {
    /// Swaps an additional player into the legacy single-Boss field view so existing
    /// combat/skill systems can process that actor without cloning the whole world.
    /// Call it a second time with the same index to restore the primary Boss context.
    pub fn swap_coop_boss_context(&mut self, index: usize) -> bool {
        let Some(player) = self.coop_bosses.get_mut(index) else {
            return false;
        };
        macro_rules! swap_field { ($field:ident) => { std::mem::swap(&mut self.$field, &mut player.$field); }; }
        std::mem::swap(&mut self.active_boss_id, &mut player.id);
        swap_field!(boss_pos);
        swap_field!(boss_vel);
        swap_field!(boss_facing);
        swap_field!(boss_name);
        swap_field!(boss_skin);
        swap_field!(boss_health);
        swap_field!(boss_resistances);
        swap_field!(boss_inventory);
        swap_field!(boss_dash_cooldown);
        swap_field!(boss_invulnerable_timer);
        swap_field!(boss_policy_think_timer);
        swap_field!(boss_policy_has_cached_action);
        swap_field!(boss_policy_move_dir);
        swap_field!(boss_policy_move_mag);
        swap_field!(boss_policy_dash);
        swap_field!(boss_policy_dash_dir);
        swap_field!(boss_policy_rest);
        swap_field!(boss_still_time);
        swap_field!(boss_damage_taken_timer);
        swap_field!(boss_mushroom_still_time);
        swap_field!(boss_poison_timer);
        swap_field!(boss_poison_dps);
        std::mem::swap(&mut self.boss_revive_progress, &mut player.revive_progress);
        std::mem::swap(&mut self.boss_pending_level_ups, &mut player.pending_level_ups);
        std::mem::swap(&mut self.boss_pending_relic_choices, &mut player.pending_relic_choices);
        std::mem::swap(&mut self.boss_level_up_options, &mut player.level_up_options);
        std::mem::swap(&mut self.boss_human_controlled, &mut player.human_controlled);
        swap_field!(blade_squall_angle);
        swap_field!(light_bastion_angle);
        swap_field!(death_ray_active);
        true
    }

    pub fn add_coop_boss(&mut self, id: u8, pos: Vec2, name: impl Into<String>, skin: u8) {
        if id <= 1 || self.coop_bosses.iter().any(|boss| boss.id == id) {
            return;
        }
        let old_count = self.boss_player_count();
        self.coop_bosses.push(BossPlayerState::new(id, pos, name, skin));
        let new_count = self.boss_player_count();
        let scale = new_count as f32 / old_count.max(1) as f32;
        for structure in &mut self.structures {
            if structure.structure_type == crate::content::structures::STRUCTURE_OBELISK {
                structure.health.max *= scale;
                structure.health.current *= scale;
            }
        }
    }

    pub fn boss_player_count(&self) -> usize {
        self.coop_bosses.len() + 1
    }

    pub fn living_boss_count(&self) -> usize {
        usize::from(self.boss_health.is_alive())
            + self.coop_bosses.iter().filter(|player| player.boss_health.is_alive()).count()
    }

    pub fn any_boss_alive(&self) -> bool {
        self.boss_health.is_alive() || self.coop_bosses.iter().any(|player| player.boss_health.is_alive())
    }

    pub fn nearest_live_boss_pos(&self, from: Vec2) -> Vec2 {
        let mut target = self.boss_pos;
        let mut distance = if self.boss_health.is_alive() {
            from.distance_squared(target)
        } else {
            f32::MAX
        };
        for player in self.coop_bosses.iter().filter(|player| player.boss_health.is_alive()) {
            let candidate_distance = from.distance_squared(player.boss_pos);
            if candidate_distance < distance {
                target = player.boss_pos;
                distance = candidate_distance;
            }
        }
        target
    }

    pub fn record_structure_contribution_for(&mut self, boss_id: u8, structure_id: usize) {
        self.structure_contributors
            .entry(structure_id)
            .or_default()
            .insert(boss_id);
    }

    pub fn record_structure_contribution(&mut self, structure_id: usize) {
        self.record_structure_contribution_for(self.active_boss_id, structure_id);
    }

    pub fn record_obelisk_contribution_for(&mut self, boss_id: u8, structure_id: usize) {
        self.record_structure_contribution_for(boss_id, structure_id);
        self.obelisk_contributors
            .entry(structure_id)
            .or_default()
            .insert(boss_id);
    }

    pub fn record_obelisk_contribution(&mut self, structure_id: usize) {
        self.record_obelisk_contribution_for(self.active_boss_id, structure_id);
    }

    #[inline]
    pub fn boss_pos_by_id(&self, boss_id: u8) -> Option<Vec2> {
        if self.active_boss_id == boss_id {
            Some(self.boss_pos)
        } else {
            self.coop_bosses
                .iter()
                .find(|player| player.id == boss_id)
                .map(|player| player.boss_pos)
        }
    }

    #[inline]
    pub fn boss_inventory_by_id(&self, boss_id: u8) -> Option<&BossInventory> {
        if self.active_boss_id == boss_id {
            Some(&self.boss_inventory)
        } else {
            self.coop_bosses
                .iter()
                .find(|player| player.id == boss_id)
                .map(|player| &player.boss_inventory)
        }
    }

    #[inline]
    pub fn heal_boss_by_id(&mut self, boss_id: u8, amount: f32) {
        if amount <= 0.0 {
            return;
        }
        if self.active_boss_id == boss_id {
            self.boss_health.heal(amount);
        } else if let Some(player) = self.coop_bosses.iter_mut().find(|p| p.id == boss_id) {
            player.boss_health.heal(amount);
        }
    }

    pub fn grant_boss_player_xp(&mut self, player_id: u8, amount: f32) {
        if self.active_boss_id == player_id {
            if self.boss_health.is_alive() {
                self.boss_pending_level_ups += self.boss_inventory.add_xp(amount);
            }
        } else if let Some(player) = self.coop_bosses.iter_mut().find(|player| player.id == player_id) {
            if player.boss_health.is_alive() {
                player.pending_level_ups += player.boss_inventory.add_xp(amount);
            }
        }
    }

    /// Распределяет опыт между живыми Боссами кратно их количеству.
    /// Поверженные боссы не получают опыт до воскрешения.
    pub fn grant_shared_boss_xp(&mut self, total_xp: f32) {
        if total_xp <= 0.0 {
            return;
        }
        let living_count = self.living_boss_count();
        if living_count == 0 {
            return;
        }
        let share = total_xp / living_count as f32;
        if self.boss_health.is_alive() {
            self.boss_pending_level_ups += self.boss_inventory.add_xp(share);
        }
        for player in &mut self.coop_bosses {
            if player.boss_health.is_alive() {
                player.pending_level_ups += player.boss_inventory.add_xp(share);
            }
        }
    }

    pub fn push_audio(&mut self, event: crate::audio::AudioEvent) {
        if self.audio_events.len() < 32 {
            self.audio_events.push(event);
        }
    }

    pub fn new() -> Self {
        Self {
            boss_pos: Vec2::new(crate::core::MAP_WIDTH * 0.5, crate::core::MAP_HEIGHT * 0.5),
            boss_vel: Vec2::ZERO,
            boss_facing: Vec2::X,
            boss_name: "Игрок".to_string(),
            boss_skin: 0,
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
            boss_revive_progress: 0.0,
            boss_human_controlled: false,
            boss_input_move_dir: Vec2::ZERO,
            boss_input_aim_dir: Vec2::X,
            boss_input_dash_pressed: false,
            boss_pending_level_ups: 0,
            boss_pending_relic_choices: 0,
            boss_level_up_options: None,
            active_boss_id: 1,
            blade_squall_angle: 0.0,
            light_bastion_angle: 0.0,
            death_ray_active: None,
            coop_bosses: Vec::new(),
            obelisk_contributors: HashMap::new(),
            structure_contributors: HashMap::new(),
            game_time: 0.0,
            match_duration_secs: crate::core::time::GAME_DURATION_SECS,
            spatial_grid: SpatialGrid::new(crate::core::MAP_WIDTH, crate::core::MAP_HEIGHT),
            crowd_grid: crate::core::crowd::CrowdDensityGrid::new(crate::core::MAP_WIDTH, crate::core::MAP_HEIGHT),
            mob_positions_scratch: Vec::with_capacity(4096),
            flow_field: crate::ai::flow_field::FlowField::load_prebaked(),
            mobs: Vec::with_capacity(3000),
            projectiles: Vec::with_capacity(1500),
            gems: Vec::with_capacity(3000),
            structures: Vec::new(),
            acid_pools: Vec::new(),
            black_holes: Vec::new(),
            floating_texts: Vec::with_capacity(500),
            particles: Vec::with_capacity(2000),
            lightning_arcs: Vec::new(),
            boss_waypoint_queue: VecDeque::new(),
            move_click_effects: Vec::new(),
            directors: vec![DirectorState::new(1)],
            damage_queue: Vec::new(),
            next_entity_id: 1,
            total_damage_dealt_to_boss: 0.0,
            boss_altars_destroyed_total: 0,
            boss_obelisks_deactivated_total: 0,
            screen_shake: 0.0,
            soul_link_tension: 0.0,
            obelisk_cycle_timer: 60.0,
            // Модель загружается один раз в main.rs / симуляции, а не на каждом World::new().
            boss_model: None,
            audio_events: Vec::new(),
            is_headless: false,
            is_network_paused: false,
            pause_initiator: None,
        }
    }

    pub fn spawn_move_click(&mut self, pos: Vec2) {
        self.move_click_effects.push(MoveClickEffect {
            pos,
            timer: 0.45,
            duration: 0.45,
        });
    }

    pub fn alloc_id(&mut self) -> usize {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    /// Перестраивает пространственную сетку по текущему списку мобов.
    /// Индексы в сетке валидны только до следующего удаления мобов.
    pub fn rebuild_spatial_grid(&mut self) {
        self.spatial_grid.rebuild_from_slice(&self.mobs, |mob| mob.pos);
    }

    pub fn add_floating_text(&mut self, pos: Vec2, text: String, color: Color, scale: f32) {
        if self.is_headless {
            return;
        }
        if self.floating_texts.len() < 300 {
            self.floating_texts.push(FloatingTextEntity {
                pos: pos + Vec2::new(crate::core::visual_rand::gen_range(-10.0, 10.0), crate::core::visual_rand::gen_range(-10.0, 5.0)),
                vel: Vec2::new(crate::core::visual_rand::gen_range(-15.0, 15.0), -45.0),
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
                let angle = crate::core::visual_rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                let speed = crate::core::visual_rand::gen_range(20.0, speed_range);
                let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
                let lifetime = crate::core::visual_rand::gen_range(0.2, 0.5);
                self.particles.push(ParticleEntity {
                    pos,
                    vel,
                    color,
                    radius: crate::core::visual_rand::gen_range(1.5, 3.5),
                    lifetime,
                    max_lifetime: lifetime,
                });
            }
        }
    }
}
