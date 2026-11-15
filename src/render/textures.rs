use macroquad::prelude::*;
use crate::ecs::components::combat::DamageType;
use crate::ecs::components::inventory::{PassiveType, WeaponType};

pub struct GameTextures {
    // Mobs
    pub skeleton: Option<Texture2D>,
    pub hound_frames: Vec<Texture2D>,
    pub cultist: Option<Texture2D>,
    pub golem: Option<Texture2D>,
    pub kamikaze: Option<Texture2D>,
    pub shadow_demon: Option<Texture2D>,
    pub fire_elemental: Option<Texture2D>,
    pub bringer_death: Option<Texture2D>,

    // Boss (Knight)
    pub knight_idle: Vec<Texture2D>,
    pub knight_walk: Vec<Texture2D>,

    // Structures & Items
    pub altar: Option<Texture2D>,
    pub soul_crystal: Option<Texture2D>,
    pub chest: Option<Texture2D>,

    // Environment
    pub floor_stone: Option<Texture2D>,
    pub wall_brick: Option<Texture2D>,

    // Projectiles
    pub proj_wind: Option<Texture2D>,
    pub proj_fire: Option<Texture2D>,
    pub proj_magic: Option<Texture2D>,
    pub proj_holy: Option<Texture2D>,
    pub proj_ice: Option<Texture2D>,
    pub proj_poison: Option<Texture2D>,
    pub proj_axe: Option<Texture2D>,
    pub proj_arrow: Option<Texture2D>,
    pub proj_lightning: Option<Texture2D>,
    pub proj_dagger: Option<Texture2D>,
    pub proj_scythe: Option<Texture2D>,

    // Equipment & Ability Icons
    pub icon_sword: Option<Texture2D>,
    pub icon_shield: Option<Texture2D>,
    pub icon_wand: Option<Texture2D>,
    pub icon_ax: Option<Texture2D>,
    pub icon_bow: Option<Texture2D>,
    pub icon_glove: Option<Texture2D>,
    pub icon_armor: Option<Texture2D>,
    pub icon_elixir: Option<Texture2D>,
    pub icon_book: Option<Texture2D>,
    pub icon_ring: Option<Texture2D>,
    pub icon_bottle: Option<Texture2D>,
    pub icon_chest: Option<Texture2D>,
    pub icon_hourglass: Option<Texture2D>,
    pub icon_jewel: Option<Texture2D>,
}

async fn load_filtered(path: &str, filter: FilterMode) -> Option<Texture2D> {
    if let Ok(tex) = load_texture(path).await {
        tex.set_filter(filter);
        Some(tex)
    } else {
        None
    }
}

impl GameTextures {
    pub async fn load() -> Self {
        // Pixel art sprites use Nearest filtering for sharp pixel look
        let p_filter = FilterMode::Nearest;
        // HD icons use Linear filtering for smooth anti-aliased look
        let ui_filter = FilterMode::Linear;

        let skeleton = load_filtered("assets/sprites/mob_skeleton_white.png", p_filter).await;

        let mut hound_frames = Vec::new();
        for i in 0..4 {
            if let Some(t) = load_filtered(&format!("assets/sprites/mob_hound_{}.png", i), p_filter).await {
                hound_frames.push(t);
            }
        }

        let cultist = load_filtered("assets/sprites/mob_cultist.png", p_filter).await;
        let golem = load_filtered("assets/sprites/mob_golem.png", p_filter).await;
        let kamikaze = load_filtered("assets/sprites/mob_kamikaze.png", p_filter).await;
        let shadow_demon = load_filtered("assets/sprites/mob_shadow_demon.png", p_filter).await;
        let fire_elemental = load_filtered("assets/sprites/mob_fire_elemental.png", p_filter).await;
        let bringer_death = load_filtered("assets/sprites/mob_bringer_death.png", p_filter).await;

        let mut knight_idle = Vec::new();
        for i in 0..4 {
            if let Some(t) = load_filtered(&format!("assets/sprites/knight_idle_{}.png", i), p_filter).await {
                knight_idle.push(t);
            }
        }

        let mut knight_walk = Vec::new();
        for i in 0..4 {
            if let Some(t) = load_filtered(&format!("assets/sprites/knight_walk_{}.png", i), p_filter).await {
                knight_walk.push(t);
            }
        }

        let altar = load_filtered("assets/sprites/structure_altar.png", p_filter).await;
        let soul_crystal = load_filtered("assets/sprites/crystal_soul.png", p_filter).await;
        let chest = load_filtered("assets/sprites/item_chest.png", p_filter).await;

        let floor_stone = load_filtered("assets/tiles/floor_stone.png", p_filter).await;
        let wall_brick = load_filtered("assets/tiles/wall_brick.png", p_filter).await;

        let proj_wind = load_filtered("assets/sprites/proj_wind.png", p_filter).await;
        let proj_fire = load_filtered("assets/sprites/proj_fire.png", p_filter).await;
        let proj_magic = load_filtered("assets/sprites/proj_magic.png", p_filter).await;
        let proj_holy = load_filtered("assets/sprites/proj_holy.png", p_filter).await;
        let proj_ice = load_filtered("assets/sprites/proj_ice.png", p_filter).await;
        let proj_poison = load_filtered("assets/sprites/proj_poison.png", p_filter).await;
        let proj_axe = load_filtered("assets/sprites/proj_axe.png", p_filter).await;
        let proj_arrow = load_filtered("assets/sprites/proj_arrow.png", p_filter).await;
        let proj_lightning = load_filtered("assets/sprites/proj_lightning.png", p_filter).await;
        let proj_dagger = load_filtered("assets/sprites/proj_dagger.png", p_filter).await;
        let proj_scythe = load_filtered("assets/sprites/proj_scythe.png", p_filter).await;

        let icon_sword = load_filtered("assets/icons/sword_square_transparent.png", ui_filter).await;
        let icon_shield = load_filtered("assets/icons/shield_square_transparent.png", ui_filter).await;
        let icon_wand = load_filtered("assets/icons/wand_square_transparent.png", ui_filter).await;
        let icon_ax = load_filtered("assets/icons/ax_square_transparent.png", ui_filter).await;
        let icon_bow = load_filtered("assets/icons/bow_square_transparent.png", ui_filter).await;
        let icon_glove = load_filtered("assets/icons/glove_1_square_transparent.png", ui_filter).await;
        let icon_armor = load_filtered("assets/icons/armor_1_square_transparent.png", ui_filter).await;
        let icon_elixir = load_filtered("assets/icons/elixir_1_square_transparent.png", ui_filter).await;
        let icon_book = load_filtered("assets/icons/book_square_transparent.png", ui_filter).await;
        let icon_ring = load_filtered("assets/icons/ring_1_square_transparent.png", ui_filter).await;
        let icon_bottle = load_filtered("assets/icons/bottle_square_transparent.png", ui_filter).await;
        let icon_chest = load_filtered("assets/icons/chest_1_square_transparent.png", ui_filter).await;
        let icon_hourglass = load_filtered("assets/icons/hourglass_square_transparent.png", ui_filter).await;
        let icon_jewel = load_filtered("assets/icons/jewel_square_transparent.png", ui_filter).await;

        Self {
            skeleton,
            hound_frames,
            cultist,
            golem,
            kamikaze,
            shadow_demon,
            fire_elemental,
            bringer_death,
            knight_idle,
            knight_walk,
            altar,
            soul_crystal,
            chest,
            floor_stone,
            wall_brick,
            proj_wind,
            proj_fire,
            proj_magic,
            proj_holy,
            proj_ice,
            proj_poison,
            proj_axe,
            proj_arrow,
            proj_lightning,
            proj_dagger,
            proj_scythe,
            icon_sword,
            icon_shield,
            icon_wand,
            icon_ax,
            icon_bow,
            icon_glove,
            icon_armor,
            icon_elixir,
            icon_book,
            icon_ring,
            icon_bottle,
            icon_chest,
            icon_hourglass,
            icon_jewel,
        }
    }

    pub fn get_weapon_icon(&self, weapon_type: WeaponType) -> Option<&Texture2D> {
        match weapon_type {
            WeaponType::WindBlade | WeaponType::BladeStorm => self.icon_sword.as_ref(),
            WeaponType::FireAura | WeaponType::Hellfire => self.icon_elixir.as_ref(),
            WeaponType::MagicWand | WeaponType::DeathRay => self.icon_wand.as_ref(),
            WeaponType::HolyHammer | WeaponType::DivineShield => self.icon_shield.as_ref(),
            WeaponType::IceShards | WeaponType::BlizzardStorm => self.icon_ring.as_ref(),
            WeaponType::PoisonOrbs | WeaponType::ToxicPestilence => self.icon_bottle.as_ref(),
            WeaponType::ExecutionerAxe | WeaponType::WhirlwindCleaver => self.icon_ax.as_ref(),
            WeaponType::CelestialBow | WeaponType::StarfallBarrage => self.icon_bow.as_ref(),
            WeaponType::ThunderOrb | WeaponType::ThunderstormWrath => self.icon_jewel.as_ref(),
            WeaponType::BloodDaggers | WeaponType::BloodHarvest => self.icon_sword.as_ref(),
        }
    }

    pub fn get_passive_icon(&self, passive_type: PassiveType) -> Option<&Texture2D> {
        match passive_type {
            PassiveType::SpeedGloves => self.icon_glove.as_ref(),
            PassiveType::GiantHeart => self.icon_chest.as_ref(),
            PassiveType::CooldownTome => self.icon_book.as_ref(),
            PassiveType::LightArmor => self.icon_armor.as_ref(),
            PassiveType::FrostRing => self.icon_ring.as_ref(),
            PassiveType::CorrosiveFlask => self.icon_bottle.as_ref(),
            PassiveType::BloodStone => self.icon_bottle.as_ref(),
            PassiveType::EagleScope => self.icon_ring.as_ref(),
            PassiveType::ChronoHourglass => self.icon_hourglass.as_ref(),
            PassiveType::MagnetSphere => self.icon_jewel.as_ref(),
        }
    }

    pub fn get_mob_texture(&self, tier: u8, frame: usize) -> Option<&Texture2D> {
        match tier {
            1 => self.skeleton.as_ref(),
            2 => {
                if !self.hound_frames.is_empty() {
                    Some(&self.hound_frames[frame % self.hound_frames.len()])
                } else {
                    None
                }
            }
            3 => self.cultist.as_ref(),
            4 => self.golem.as_ref(),
            5 => self.kamikaze.as_ref(),
            6 => self.shadow_demon.as_ref(),
            7 => self.fire_elemental.as_ref(),
            8 => self.bringer_death.as_ref(),
            _ => None,
        }
    }

    pub fn get_projectile_texture(&self, damage_type: DamageType) -> Option<&Texture2D> {
        match damage_type {
            DamageType::Physical => self.proj_wind.as_ref(),
            DamageType::Fire => self.proj_fire.as_ref(),
            DamageType::Ice => self.proj_ice.as_ref(),
            DamageType::Holy => self.proj_holy.as_ref(),
            DamageType::Poison => self.proj_poison.as_ref(),
            DamageType::True => self.proj_magic.as_ref(),
        }
    }

    pub fn get_weapon_projectile_texture(&self, weapon: Option<WeaponType>, damage_type: DamageType) -> Option<&Texture2D> {
        if let Some(w) = weapon {
            match w {
                WeaponType::ExecutionerAxe | WeaponType::WhirlwindCleaver => return self.proj_axe.as_ref(),
                WeaponType::CelestialBow | WeaponType::StarfallBarrage => return self.proj_arrow.as_ref(),
                WeaponType::ThunderOrb | WeaponType::ThunderstormWrath => return self.proj_lightning.as_ref(),
                WeaponType::BloodDaggers | WeaponType::BloodHarvest => return self.proj_dagger.as_ref(),
                _ => {}
            }
        }
        self.get_projectile_texture(damage_type)
    }
}
