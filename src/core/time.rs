pub const GAME_DURATION_SECS: f32 = 1020.0; // 17 минут
pub const FIXED_SIM_DT: f32 = 0.05; // 20 Гц авторитетный шаг симуляции
pub const DIRECTOR_PERIOD_TICKS: u32 = 8; // 8 * 0.05 = 0.40 сек между тактическими решениями Директора

#[derive(Debug, Clone)]
pub struct GameClock {
    pub elapsed_secs: f32,
    pub delta_secs: f32,
    pub duration_secs: f32,
}

impl GameClock {
    pub fn new() -> Self {
        Self::with_duration(GAME_DURATION_SECS)
    }

    pub fn with_duration(duration_secs: f32) -> Self {
        let duration_secs = if duration_secs.is_finite() {
            duration_secs.clamp(60.0, 10_800.0)
        } else {
            GAME_DURATION_SECS
        };
        Self {
            elapsed_secs: 0.0,
            delta_secs: 0.0,
            duration_secs,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.delta_secs = dt;
        self.elapsed_secs += dt;
    }

    pub fn time_remaining_secs(&self) -> f32 {
        (self.duration_secs - self.elapsed_secs).max(0.0)
    }

    pub fn is_time_up(&self) -> bool {
        self.elapsed_secs >= self.duration_secs
    }

    pub fn formatted_remaining(&self) -> String {
        let remaining = self.time_remaining_secs() as u32;
        let mins = remaining / 60;
        let secs = remaining % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    pub fn formatted_elapsed(&self) -> String {
        let elapsed = self.elapsed_secs as u32;
        let mins = elapsed / 60;
        let secs = elapsed % 60;
        format!("{:02}:{:02}", mins, secs)
    }
}
