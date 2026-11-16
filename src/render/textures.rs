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
    pub soul_ethereal: Option<Texture2D>,
    pub soul_ethereal_sm: Option<Texture2D>,
    pub chest: Option<Texture2D>,

    // Environment
    pub floor_stone: Option<Texture2D>,
    pub wall_brick: Option<Texture2D>,
    pub floor_dungeon_plain: Option<Texture2D>,
    pub floor_dungeon_cracked: Option<Texture2D>,
    pub floor_dungeon_blood: Option<Texture2D>,
    pub floor_dungeon_runic: Option<Texture2D>,

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
    pub icon_fire_aura: Option<Texture2D>,
    pub icon_holy_hammer: Option<Texture2D>,
    pub icon_ice_shards: Option<Texture2D>,
    pub icon_thunder: Option<Texture2D>,
    pub icon_blood_daggers: Option<Texture2D>,
    pub icon_giant_heart: Option<Texture2D>,
    pub icon_blood_stone: Option<Texture2D>,
    pub icon_eagle_scope: Option<Texture2D>,
    pub icon_magnet_sphere: Option<Texture2D>,
    pub icon_corrosive_flask: Option<Texture2D>,
    pub icon_singularity: Option<Texture2D>,
    pub icon_chain_frost: Option<Texture2D>,
    pub icon_brimstone: Option<Texture2D>,
    pub icon_missiles: Option<Texture2D>,
    pub icon_tesla_coil: Option<Texture2D>,
    pub icon_clover: Option<Texture2D>,
    pub icon_prismatic: Option<Texture2D>,
    pub icon_aegis: Option<Texture2D>,
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
        let soul_ethereal = load_filtered("assets/sprites/soul_ethereal.png", ui_filter).await;
        let soul_ethereal_sm = load_filtered("assets/sprites/soul_ethereal_sm.png", ui_filter).await;
        let chest = load_filtered("assets/sprites/item_chest.png", p_filter).await;

        let floor_stone = load_filtered("assets/tiles/floor_stone.png", p_filter).await;
        let wall_brick = load_filtered("assets/tiles/wall_brick.png", p_filter).await;
        let floor_dungeon_plain = load_filtered("assets/tiles/floor_dungeon_plain.png", p_filter).await;
        let floor_dungeon_cracked = load_filtered("assets/tiles/floor_dungeon_cracked.png", p_filter).await;
        let floor_dungeon_blood = load_filtered("assets/tiles/floor_dungeon_blood.png", p_filter).await;
        let floor_dungeon_runic = load_filtered("assets/tiles/floor_dungeon_runic.png", p_filter).await;

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
        let icon_fire_aura = load_filtered("assets/icons/icon_fire_aura.png", ui_filter).await;
        let icon_holy_hammer = load_filtered("assets/icons/icon_holy_hammer.png", ui_filter).await;
        let icon_ice_shards = load_filtered("assets/icons/icon_ice_shards.png", ui_filter).await;
        let icon_thunder = load_filtered("assets/icons/icon_thunder.png", ui_filter).await;
        let icon_blood_daggers = load_filtered("assets/icons/icon_blood_daggers.png", ui_filter).await;
        let icon_giant_heart = load_filtered("assets/icons/icon_giant_heart.png", ui_filter).await;
        let icon_blood_stone = load_filtered("assets/icons/icon_blood_stone.png", ui_filter).await;
        let icon_eagle_scope = load_filtered("assets/icons/icon_eagle_scope.png", ui_filter).await;
        let icon_magnet_sphere = load_filtered("assets/icons/icon_magnet_sphere.png", ui_filter).await;
        let icon_corrosive_flask = load_filtered("assets/icons/icon_corrosive_flask.png", ui_filter).await;
        let icon_singularity = load_filtered("assets/icons/icon_singularity.png", ui_filter).await;
        let icon_chain_frost = load_filtered("assets/icons/icon_chain_frost.png", ui_filter).await;
        let icon_brimstone = load_filtered("assets/icons/icon_brimstone.png", ui_filter).await;
        let icon_missiles = load_filtered("assets/icons/icon_missiles.png", ui_filter).await;
        let icon_tesla_coil = load_filtered("assets/icons/icon_tesla_coil.png", ui_filter).await;
        let icon_clover = load_filtered("assets/icons/icon_clover.png", ui_filter).await;
        let icon_prismatic = load_filtered("assets/icons/icon_prismatic.png", ui_filter).await;
        let icon_aegis = load_filtered("assets/icons/icon_aegis.png", ui_filter).await;

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
            soul_ethereal,
            soul_ethereal_sm,
            chest,
            floor_stone,
            wall_brick,
            floor_dungeon_plain,
            floor_dungeon_cracked,
            floor_dungeon_blood,
            floor_dungeon_runic,
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
            icon_fire_aura,
            icon_holy_hammer,
            icon_ice_shards,
            icon_thunder,
            icon_blood_daggers,
            icon_giant_heart,
            icon_blood_stone,
            icon_eagle_scope,
            icon_magnet_sphere,
            icon_corrosive_flask,
            icon_singularity,
            icon_chain_frost,
            icon_brimstone,
            icon_missiles,
            icon_tesla_coil,
            icon_clover,
            icon_prismatic,
            icon_aegis,
        }
    }

    pub fn get_weapon_icon(&self, weapon_type: WeaponType) -> Option<&Texture2D> {
        match weapon_type {
            WeaponType::WindBlade | WeaponType::BladeStorm => self.icon_sword.as_ref(),
            WeaponType::FireAura | WeaponType::Hellfire => self.icon_fire_aura.as_ref().or(self.icon_elixir.as_ref()),
            WeaponType::MagicWand | WeaponType::DeathRay => self.icon_wand.as_ref(),
            WeaponType::HolyHammer | WeaponType::DivineShield => self.icon_holy_hammer.as_ref().or(self.icon_shield.as_ref()),
            WeaponType::IceShards | WeaponType::BlizzardStorm => self.icon_ice_shards.as_ref().or(self.icon_ring.as_ref()),
            WeaponType::PoisonOrbs | WeaponType::ToxicPestilence => self.icon_corrosive_flask.as_ref().or(self.icon_bottle.as_ref()),
            WeaponType::ExecutionerAxe | WeaponType::WhirlwindCleaver => self.icon_ax.as_ref(),
            WeaponType::CelestialBow | WeaponType::StarfallBarrage => self.icon_bow.as_ref(),
            WeaponType::ThunderOrb | WeaponType::ThunderstormWrath => self.icon_thunder.as_ref().or(self.icon_jewel.as_ref()),
            WeaponType::BloodDaggers | WeaponType::BloodHarvest => self.icon_blood_daggers.as_ref().or(self.icon_sword.as_ref()),
            WeaponType::SingularityOrb | WeaponType::EventHorizon => self.icon_singularity.as_ref().or(self.icon_jewel.as_ref()),
            WeaponType::ChainFrost | WeaponType::AbsoluteZero => self.icon_chain_frost.as_ref().or(self.icon_ring.as_ref()),
            WeaponType::BrimstoneRay | WeaponType::InfernalCataclysm => self.icon_brimstone.as_ref().or(self.icon_fire_aura.as_ref()),
            WeaponType::MissileSwarm | WeaponType::DoomsdayBarrage => self.icon_missiles.as_ref().or(self.icon_bow.as_ref()),
        }
    }

    pub fn get_passive_icon(&self, passive_type: PassiveType) -> Option<&Texture2D> {
        match passive_type {
            PassiveType::SpeedGloves => self.icon_glove.as_ref(),
            PassiveType::GiantHeart => self.icon_giant_heart.as_ref().or(self.icon_chest.as_ref()),
            PassiveType::CooldownTome => self.icon_book.as_ref(),
            PassiveType::LightArmor => self.icon_armor.as_ref(),
            PassiveType::FrostRing => self.icon_ring.as_ref(),
            PassiveType::CorrosiveFlask => self.icon_corrosive_flask.as_ref().or(self.icon_bottle.as_ref()),
            PassiveType::BloodStone => self.icon_blood_stone.as_ref().or(self.icon_jewel.as_ref()),
            PassiveType::EagleScope => self.icon_eagle_scope.as_ref().or(self.icon_ring.as_ref()),
            PassiveType::ChronoHourglass => self.icon_hourglass.as_ref(),
            PassiveType::MagnetSphere => self.icon_magnet_sphere.as_ref().or(self.icon_jewel.as_ref()),
            PassiveType::TeslaCoil => self.icon_tesla_coil.as_ref().or(self.icon_jewel.as_ref()),
            PassiveType::CloverOfFate => self.icon_clover.as_ref().or(self.icon_jewel.as_ref()),
            PassiveType::PrismaticFocus => self.icon_prismatic.as_ref().or(self.icon_ring.as_ref()),
            PassiveType::TitanAegis => self.icon_aegis.as_ref().or(self.icon_armor.as_ref()),
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
