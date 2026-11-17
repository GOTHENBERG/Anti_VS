pub const GAME_DURATION_SECS: f32 = 1020.0; // 17 минут

#[derive(Debug, Clone)]
pub struct GameClock {
    pub elapsed_secs: f32,
    pub delta_secs: f32,
}

impl GameClock {
    pub fn new() -> Self {
        Self {
            elapsed_secs: 0.0,
            delta_secs: 0.0,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.delta_secs = dt;
        self.elapsed_secs += dt;
    }

    pub fn time_remaining_secs(&self) -> f32 {
        (GAME_DURATION_SECS - self.elapsed_secs).max(0.0)
    }

    pub fn is_time_up(&self) -> bool {
        self.elapsed_secs >= GAME_DURATION_SECS
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
