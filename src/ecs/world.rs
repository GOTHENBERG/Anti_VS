use macroquad::prelude::*;
use crate::ecs::components::combat::{DamagePacket, Faction, Health, Resistances};
use crate::ecs::components::inventory::BossInventory;

#[derive(Clone, Debug)]
pub struct MobEntity {
    pub id: usize,
    pub pos: Vec2,
    pub speed: f32,
    pub health: Health,
    pub resistances: Resistances,
    pub damage: f32,
    pub tier: u8,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct ProjectileEntity {
    pub id: usize,
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub lifetime: f32,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct SoulGemEntity {
    pub id: usize,
    pub pos: Vec2,
    pub xp_value: f32,
}

#[derive(Clone, Debug)]
pub struct StructureEntity {
    pub id: usize,
    pub pos: Vec2,
    pub health: Health,
    pub resistances: Resistances,
    pub faction: Faction,
    pub radius: f32,
}

#[derive(Clone, Debug)]
pub struct DirectorState {
    pub id: u8,
    pub points: f32,
    pub income_rate: f32, // Очков каждые 2 секунды
    pub total_points_spent: f32,
    pub income_timer: f32,
    pub squad_cooldowns: [f32; 5],
}

impl DirectorState {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            points: 50.0,
            income_rate: 10.0,
            total_points_spent: 0.0,
            income_timer: 0.0,
            squad_cooldowns: [0.0; 5],
        }
    }

    pub fn spend_points(&mut self, cost: f32) -> bool {
        if self.points >= cost {
            self.points -= cost;
            self.total_points_spent += cost;
            // Spend to Grow: каждые потраченные 20 очков повышают постоянный доход на +1.0
            self.income_rate += cost * 0.05;
            true
        } else {
            false
        }
    }
}

pub struct World {
    // Босс
    pub boss_pos: Vec2,
    pub boss_vel: Vec2,
    pub boss_health: Health,
    pub boss_resistances: Resistances,
    pub boss_inventory: BossInventory,
    pub boss_dash_cooldown: f32,
    pub boss_still_time: f32, // Время стояния на месте (для медитации)

    // Мобы и снаряды (плоские массивы для Data-Oriented кэша)
    pub mobs: Vec<MobEntity>,
    pub projectiles: Vec<ProjectileEntity>,
    pub gems: Vec<SoulGemEntity>,
    pub structures: Vec<StructureEntity>,

    // Директора
    pub directors: Vec<DirectorState>,

    // Очередь урона
    pub damage_queue: Vec<DamagePacket>,

    pub next_entity_id: usize,
}

impl World {
    pub fn new() -> Self {
        Self {
            boss_pos: Vec2::new(600.0, 400.0),
            boss_vel: Vec2::ZERO,
            boss_health: Health::new(1000.0),
            boss_resistances: Resistances {
                flat_armor: 5.0,
                physical_res: 0.1,
                fire_res: 0.0,
                ice_res: 0.0,
                poison_res: 0.0,
                holy_res: 0.0,
            },
            boss_inventory: BossInventory::new(),
            boss_dash_cooldown: 0.0,
            boss_still_time: 0.0,
            mobs: Vec::with_capacity(2000),
            projectiles: Vec::with_capacity(1000),
            gems: Vec::with_capacity(2000),
            structures: Vec::new(),
            directors: vec![DirectorState::new(1)],
            damage_queue: Vec::new(),
            next_entity_id: 1,
        }
    }

    pub fn alloc_id(&mut self) -> usize {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }
}
