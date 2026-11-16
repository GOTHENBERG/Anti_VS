use macroquad::audio::{load_sound, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioEvent {
    BossHit,
    BossDash,
    BossLevelUp,
    BossDeath,
    BladeSlash,
    FireBlast,
    LaserBeam,
    MagicWand,
    LightningStrike,
    IceFreeze,
    RocketSwarm,
    SummonMob,
    AltarBuild,
    UpgradeGrade,
    UiClick,
    BookOpen,
    BookClose,
}

pub struct AudioManager {
    pub music_battle: Option<Sound>,
    pub boss_hit: Option<Sound>,
    pub boss_dash: Option<Sound>,
    pub boss_level_up: Option<Sound>,
    pub boss_death: Option<Sound>,
    pub blade_slash: Option<Sound>,
    pub fire_blast: Option<Sound>,
    pub laser_beam: Option<Sound>,
    pub magic_wand: Option<Sound>,
    pub lightning_strike: Option<Sound>,
    pub ice_freeze: Option<Sound>,
    pub rocket_swarm: Option<Sound>,
    pub summon_mob: Option<Sound>,
    pub altar_build: Option<Sound>,
    pub upgrade_grade: Option<Sound>,
    pub ui_click: Option<Sound>,
    pub book_open: Option<Sound>,
    pub book_close: Option<Sound>,
    pub is_music_playing: bool,
    pub is_muted: bool,
    pub sfx_volume: f32,    // По умолчанию 10% (0.10)
    pub music_volume: f32,  // По умолчанию 3% (0.03)
}

impl AudioManager {
    pub async fn load() -> Self {
        let music_battle = load_sound("assets/audio/music_battle.ogg").await.ok();
        let boss_hit = load_sound("assets/audio/boss_hit.ogg").await.ok();
        let boss_dash = load_sound("assets/audio/boss_dash.wav").await.ok();
        let boss_level_up = load_sound("assets/audio/boss_level_up.ogg").await.ok();
        let boss_death = load_sound("assets/audio/boss_death.ogg").await.ok();
        let blade_slash = load_sound("assets/audio/blade_slash.ogg").await.ok();
        let fire_blast = load_sound("assets/audio/fire_blast.ogg").await.ok();
        let laser_beam = load_sound("assets/audio/laser_beam.ogg").await.ok();
        let magic_wand = load_sound("assets/audio/magic_wand.ogg").await.ok();
        let lightning_strike = load_sound("assets/audio/lightning_strike.ogg").await.ok();
        let ice_freeze = load_sound("assets/audio/ice_freeze.ogg").await.ok();
        let rocket_swarm = load_sound("assets/audio/rocket_swarm.ogg").await.ok();
        let summon_mob = load_sound("assets/audio/summon_mob.ogg").await.ok();
        let altar_build = load_sound("assets/audio/altar_build.ogg").await.ok();
        let upgrade_grade = load_sound("assets/audio/upgrade_grade.ogg").await.ok();
        let ui_click = load_sound("assets/audio/ui_click.wav").await.ok();
        let book_open = load_sound("assets/audio/book_open.ogg").await.ok();
        let book_close = load_sound("assets/audio/book_close.ogg").await.ok();

        Self {
            music_battle,
            boss_hit,
            boss_dash,
            boss_level_up,
            boss_death,
            blade_slash,
            fire_blast,
            laser_beam,
            magic_wand,
            lightning_strike,
            ice_freeze,
            rocket_swarm,
            summon_mob,
            altar_build,
            upgrade_grade,
            ui_click,
            book_open,
            book_close,
            is_music_playing: false,
            is_muted: false,
            sfx_volume: 0.10,   // Звуки по умолчанию на 10%
            music_volume: 0.03, // Музыка по умолчанию на 3%
        }
    }

    pub fn start_music(&mut self) {
        if let Some(sound) = &self.music_battle {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: true,
                    volume: if self.is_muted { 0.0 } else { self.music_volume },
                },
            );
            self.is_music_playing = true;
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(sound) = &self.music_battle {
            stop_sound(sound);
            self.is_music_playing = false;
        }
    }

    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
        if let Some(sound) = &self.music_battle {
            if self.is_muted {
                set_sound_volume(sound, 0.0);
            } else {
                if !self.is_music_playing {
                    self.start_music();
                } else {
                    set_sound_volume(sound, self.music_volume);
                }
            }
        }
    }

    pub fn set_music_volume(&mut self, vol: f32) {
        self.music_volume = vol.clamp(0.0, 1.0);
        if let Some(sound) = &self.music_battle {
            if self.is_music_playing && !self.is_muted {
                set_sound_volume(sound, self.music_volume);
            }
        }
    }

    pub fn set_sfx_volume(&mut self, vol: f32) {
        self.sfx_volume = vol.clamp(0.0, 1.0);
    }

    /// Пространственное позиционирование музыки: джаз исходит от босса.
    /// Чем ближе камера к боссу, тем ярче и сочнее звучит джаз;
    /// при удалении камеры на край арены музыка плавно затухает.
    pub fn update_spatial_music(&mut self, cam_pos: macroquad::prelude::Vec2, boss_pos: macroquad::prelude::Vec2) {
        if self.is_muted || !self.is_music_playing { return; }
        let dist = cam_pos.distance(boss_pos);
        let max_dist = 1800.0;
        let factor = if dist <= 300.0 {
            1.0
        } else {
            let t = ((dist - 300.0) / (max_dist - 300.0)).clamp(0.0, 1.0);
            1.0 - t * 0.82
        };
        let effective_vol = (self.music_volume * factor).clamp(0.0, 1.0);
        if let Some(sound) = &self.music_battle {
            set_sound_volume(sound, effective_vol);
        }
    }

    pub fn handle_event(&self, event: AudioEvent) {
        if self.is_muted { return; }
        match event {
            AudioEvent::BossHit => {}, // Отключено: избыточный шум при столкновениях
            AudioEvent::BossDash => self.play_sfx(self.boss_dash.as_ref(), 0.70),
            AudioEvent::BossLevelUp => self.play_sfx(self.boss_level_up.as_ref(), 0.90),
            AudioEvent::BossDeath => self.play_sfx(self.boss_death.as_ref(), 0.85),
            AudioEvent::BladeSlash => self.play_sfx(self.blade_slash.as_ref(), 0.55),
            AudioEvent::FireBlast => self.play_sfx(self.fire_blast.as_ref(), 0.65),
            AudioEvent::LaserBeam => self.play_sfx(self.laser_beam.as_ref(), 0.60),
            AudioEvent::MagicWand => self.play_sfx(self.magic_wand.as_ref(), 0.55),
            AudioEvent::LightningStrike => self.play_sfx(self.lightning_strike.as_ref(), 0.65),
            AudioEvent::IceFreeze => self.play_sfx(self.ice_freeze.as_ref(), 0.65),
            AudioEvent::RocketSwarm => self.play_sfx(self.rocket_swarm.as_ref(), 0.60),
            AudioEvent::SummonMob => self.play_sfx(self.summon_mob.as_ref(), 0.70),
            AudioEvent::AltarBuild => self.play_sfx(self.altar_build.as_ref(), 0.80),
            AudioEvent::UpgradeGrade => self.play_sfx(self.upgrade_grade.as_ref(), 0.75),
            AudioEvent::UiClick => self.play_sfx(self.ui_click.as_ref(), 0.50),
            AudioEvent::BookOpen => self.play_sfx(self.book_open.as_ref(), 0.70),
            AudioEvent::BookClose => self.play_sfx(self.book_close.as_ref(), 0.70),
        }
    }

    pub fn process_events(&self, world: &mut crate::ecs::world::World) {
        if self.is_muted || world.audio_events.is_empty() {
            world.audio_events.clear();
            return;
        }
        // Дедупликация звуков за один кадр: не допускает фазового искажения и клиппинга
        let mut played_types = std::collections::HashSet::new();
        for event in world.audio_events.drain(..) {
            if played_types.insert(event) {
                self.handle_event(event);
            }
        }
    }

    fn play_sfx(&self, sound: Option<&Sound>, volume: f32) {
        if let Some(s) = sound {
            let final_vol = (volume * self.sfx_volume).clamp(0.0, 1.0);
            if final_vol > 0.0005 {
                play_sound(
                    s,
                    PlaySoundParams {
                        looped: false,
                        volume: final_vol,
                    },
                );
            }
        }
    }
}
