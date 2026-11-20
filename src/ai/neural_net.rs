use crate::core::rand;
use serde::{Deserialize, Serialize};

// ============================================================================
// БОСС: 260 сенсоров -> 64 -> 48 -> 7 действий
// ============================================================================
pub const INPUT_SIZE: usize = 260;
pub const HIDDEN1_SIZE: usize = 64;
pub const HIDDEN2_SIZE: usize = 48;
pub const OUTPUT_SIZE: usize = 7;
pub const BOSS_DECISION_INTERVAL: f32 = 0.2;
pub const BOSS_TRAINING_DECISION_INTERVAL: f32 = 0.2;
pub const BOSS_DASH_BASE_COOLDOWN_SECONDS: f32 = 5.0;

// Семантика выходов Босса:
// [0..2]  направление движения (tanh)
// [2]     ходьба/стоп (sigmoid > 0.5) — только максимальная скорость или ноль
// [3]     триггер рывка (sigmoid > 0.5)
// [4..6]  направление рывка (tanh), при нулевом векторе используется направление движения
// [6]     намерение замереть / медитировать (sigmoid > 0.5)
pub const OUT_MOVE_X: usize = 0;
pub const OUT_MOVE_Y: usize = 1;
pub const OUT_MOVE_MAG: usize = 2;
pub const OUT_DASH: usize = 3;
pub const OUT_DASH_X: usize = 4;
pub const OUT_DASH_Y: usize = 5;
pub const OUT_REST: usize = 6;

/// Полносвязная глубокая нейронная сеть (Deep MLP)
/// Архитектура: 260 сенсорных входов -> 64 (Leaky ReLU) -> 48 (Leaky ReLU) -> 7 выходов.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuralNet {
    pub input_size: usize,
    pub hidden1_size: usize,
    pub hidden2_size: usize,
    pub output_size: usize,

    // Слой 1: 182 -> 64
    pub w1: Vec<Vec<f32>>,
    pub b1: Vec<f32>,

    // Слой 2: 64 -> 48
    pub w2: Vec<Vec<f32>>,
    pub b2: Vec<f32>,

    // Слой 3: 48 -> 7
    pub w3: Vec<Vec<f32>>,
    pub b3: Vec<f32>,
}

impl NeuralNet {
    /// Создаёт нейронную сеть с инициализацией весов по методу Хе (He / Kaiming init)
    pub fn new() -> Self {
        let scale1 = (2.0 / INPUT_SIZE as f32).sqrt();
        let scale2 = (2.0 / HIDDEN1_SIZE as f32).sqrt();
        let scale3 = (2.0 / HIDDEN2_SIZE as f32).sqrt();

        let w1 = (0..HIDDEN1_SIZE)
            .map(|_| (0..INPUT_SIZE).map(|_| rand::gen_range(-scale1, scale1)).collect())
            .collect();
        let b1 = vec![0.0; HIDDEN1_SIZE];

        let w2 = (0..HIDDEN2_SIZE)
            .map(|_| (0..HIDDEN1_SIZE).map(|_| rand::gen_range(-scale2, scale2)).collect())
            .collect();
        let b2 = vec![0.0; HIDDEN2_SIZE];

        let w3 = (0..OUTPUT_SIZE)
            .map(|_| (0..HIDDEN2_SIZE).map(|_| rand::gen_range(-scale3, scale3)).collect())
            .collect();
        let b3 = vec![0.0; OUTPUT_SIZE];

        Self {
            input_size: INPUT_SIZE,
            hidden1_size: HIDDEN1_SIZE,
            hidden2_size: HIDDEN2_SIZE,
            output_size: OUTPUT_SIZE,
            w1,
            b1,
            w2,
            b2,
            w3,
            b3,
        }
    }

    /// Проверяет, что размеры матриц соответствуют текущей архитектуре.
    /// Нужно для безопасной загрузки старых/битых JSON-моделей.
    pub fn upgrade_input_schema(&mut self) -> bool {
        let old_input = self.input_size;
        let old_shape_ok = old_input > 0
            && old_input <= INPUT_SIZE
            && self.hidden1_size == HIDDEN1_SIZE
            && self.hidden2_size == HIDDEN2_SIZE
            && self.output_size == OUTPUT_SIZE
            && self.w1.len() == HIDDEN1_SIZE
            && self.w1.iter().all(|r| r.len() == old_input)
            && self.b1.len() == HIDDEN1_SIZE
            && self.w2.len() == HIDDEN2_SIZE
            && self.w2.iter().all(|r| r.len() == HIDDEN1_SIZE)
            && self.b2.len() == HIDDEN2_SIZE
            && self.w3.len() == OUTPUT_SIZE
            && self.w3.iter().all(|r| r.len() == HIDDEN2_SIZE)
            && self.b3.len() == OUTPUT_SIZE;
        if !old_shape_ok {
            return false;
        }

        if old_input < INPUT_SIZE {
            for row in &mut self.w1 {
                row.resize(INPUT_SIZE, 0.0);
            }
            self.input_size = INPUT_SIZE;
        }
        self.is_compatible()
    }

    pub fn is_compatible(&self) -> bool {
        self.input_size == INPUT_SIZE
            && self.hidden1_size == HIDDEN1_SIZE
            && self.hidden2_size == HIDDEN2_SIZE
            && self.output_size == OUTPUT_SIZE
            && self.w1.len() == HIDDEN1_SIZE
            && self.w1.iter().all(|r| r.len() == INPUT_SIZE)
            && self.b1.len() == HIDDEN1_SIZE
            && self.w2.len() == HIDDEN2_SIZE
            && self.w2.iter().all(|r| r.len() == HIDDEN1_SIZE)
            && self.b2.len() == HIDDEN2_SIZE
            && self.w3.len() == OUTPUT_SIZE
            && self.w3.iter().all(|r| r.len() == HIDDEN2_SIZE)
            && self.b3.len() == OUTPUT_SIZE
            && self.w1.iter().chain(&self.w2).chain(&self.w3).all(|row| {
                row.iter().all(|w| w.is_finite())
            })
            && self
                .b1
                .iter()
                .chain(&self.b2)
                .chain(&self.b3)
                .all(|b| b.is_finite())
    }

    /// Прямой проход нейросети (Forward inference pass).
    /// Векторизован через SIMD (без bounds checks) и выполняется за микросекунды без аллокаций памяти.
    pub fn forward(&self, inputs: &[f32; INPUT_SIZE]) -> [f32; OUTPUT_SIZE] {
        // 1. Скрытый слой 1 (Linear + Leaky ReLU)
        let mut h1 = [0.0f32; HIDDEN1_SIZE];
        for (i, row) in self.w1.iter().enumerate() {
            let mut sum = self.b1[i];
            for (&w, &inp) in row.iter().zip(inputs.iter()) {
                sum += w * inp;
            }
            h1[i] = if sum > 0.0 { sum } else { 0.01 * sum };
        }

        // 2. Скрытый слой 2 (Linear + Leaky ReLU)
        let mut h2 = [0.0f32; HIDDEN2_SIZE];
        for (i, row) in self.w2.iter().enumerate() {
            let mut sum = self.b2[i];
            for (&w, &h) in row.iter().zip(h1.iter()) {
                sum += w * h;
            }
            h2[i] = if sum > 0.0 { sum } else { 0.01 * sum };
        }

        // 3. Выходной слой
        let mut out = [0.0f32; OUTPUT_SIZE];
        for (i, row) in self.w3.iter().enumerate() {
            let mut sum = self.b3[i];
            for (&w, &h) in row.iter().zip(h2.iter()) {
                sum += w * h;
            }
            out[i] = match i {
                OUT_MOVE_X | OUT_MOVE_Y | OUT_DASH_X | OUT_DASH_Y => sum.tanh(),
                _ => 1.0 / (1.0 + (-sum).exp()),
            };
        }

        out
    }

    /// Мутация синаптических весов для эволюции с масштабированием шума по слоям (Fan-in scaling).
    /// Защищает скрытые представления глубоких слоев от взрыва дисперсии.
    pub fn mutate(&mut self, rate: f32, sigma: f32) {
        let s1 = sigma * (48.0 / INPUT_SIZE as f32).sqrt();
        let s2 = sigma * (48.0 / HIDDEN1_SIZE as f32).sqrt();
        let s3 = sigma;

        for row in self.w1.iter_mut() {
            for w in row.iter_mut() {
                if rand::gen_range(0.0, 1.0) < rate {
                    *w += rand::gen_range(-s1, s1);
                }
            }
        }
        for b in self.b1.iter_mut() {
            if rand::gen_range(0.0, 1.0) < rate {
                *b += rand::gen_range(-s1, s1);
            }
        }

        for row in self.w2.iter_mut() {
            for w in row.iter_mut() {
                if rand::gen_range(0.0, 1.0) < rate {
                    *w += rand::gen_range(-s2, s2);
                }
            }
        }
        for b in self.b2.iter_mut() {
            if rand::gen_range(0.0, 1.0) < rate {
                *b += rand::gen_range(-s2, s2);
            }
        }

        for row in self.w3.iter_mut() {
            for w in row.iter_mut() {
                if rand::gen_range(0.0, 1.0) < rate {
                    *w += rand::gen_range(-s3, s3);
                }
            }
        }
        for b in self.b3.iter_mut() {
            if rand::gen_range(0.0, 1.0) < rate {
                *b += rand::gen_range(-s3, s3);
            }
        }
    }

    /// L2 weight decay: тормозит «раздувание» крупных весов и насыщение tanh/sigmoid.
    /// Не трогает малые веса (|w| <= 1.5), сохраняя полезную тонкую структуру связей.
    pub fn decay(&mut self, factor: f32) {
        let k = (1.0 - factor).clamp(0.0, 1.0);
        for row in self.w1.iter_mut() {
            for w in row.iter_mut() {
                if w.abs() > 1.5 {
                    *w *= k;
                }
            }
        }
        for row in self.w2.iter_mut() {
            for w in row.iter_mut() {
                if w.abs() > 1.5 {
                    *w *= k;
                }
            }
        }
        for row in self.w3.iter_mut() {
            for w in row.iter_mut() {
                if w.abs() > 1.5 {
                    *w *= k;
                }
            }
        }
    }

    /// Генетический кроссовер весовых матриц двух нейросетей
    pub fn crossover(&self, other: &Self) -> Self {
        let mut child = self.clone();

        for i in 0..HIDDEN1_SIZE {
            for j in 0..INPUT_SIZE {
                if rand::gen_range(0.0, 1.0) < 0.5 {
                    child.w1[i][j] = other.w1[i][j];
                }
            }
            if rand::gen_range(0.0, 1.0) < 0.5 {
                child.b1[i] = other.b1[i];
            }
        }

        for i in 0..HIDDEN2_SIZE {
            for j in 0..HIDDEN1_SIZE {
                if rand::gen_range(0.0, 1.0) < 0.5 {
                    child.w2[i][j] = other.w2[i][j];
                }
            }
            if rand::gen_range(0.0, 1.0) < 0.5 {
                child.b2[i] = other.b2[i];
            }
        }

        for i in 0..OUTPUT_SIZE {
            for j in 0..HIDDEN2_SIZE {
                if rand::gen_range(0.0, 1.0) < 0.5 {
                    child.w3[i][j] = other.w3[i][j];
                }
            }
            if rand::gen_range(0.0, 1.0) < 0.5 {
                child.b3[i] = other.b3[i];
            }
        }

        child
    }

    /// Общее число параметров (синапсов) в мозге
    pub fn total_synapses(&self) -> usize {
        (INPUT_SIZE * HIDDEN1_SIZE + HIDDEN1_SIZE)
            + (HIDDEN1_SIZE * HIDDEN2_SIZE + HIDDEN2_SIZE)
            + (HIDDEN2_SIZE * OUTPUT_SIZE + OUTPUT_SIZE)
    }
}

// ============================================================================
// ДИРЕКТОР: 177 сенсоров -> 64 -> 48 -> три головы
// ============================================================================
pub const DIRECTOR_INPUT_SIZE: usize = 177;
pub const DIRECTOR_HIDDEN1_SIZE: usize = 64;
pub const DIRECTOR_HIDDEN2_SIZE: usize = 48;
pub const DIRECTOR_ECONOMY_HEAD_SIZE: usize = 20; // 0: Пас, 1..16: Тиры, 17: Ярость, 18: Панцирь, 19: Чумные споры
pub const DIRECTOR_ALTAR_HEAD_SIZE: usize = 19; // [0]: триггер, [1..2]: слот, [3..18]: тир алтаря
pub const DIRECTOR_SPAWN_HEAD_SIZE: usize = 8; // 8 секторов спавна вокруг Босса

/// Контекст для маски доступности и совместного бюджета действий Директора.
#[derive(Clone, Copy, Debug)]
pub struct DirectorActionContext {
    pub points: f32,
    pub total_points_spent: f32,
    pub altar_cooldown: f32,
    pub altar_cost: f32,
    pub altar_tier_cooldowns: [f32; 16],
    pub altar_tier_costs: [f32; 16],
    pub grade_fury: u32,
    pub grade_carapace: u32,
    pub grade_pestilence: u32,
    pub squad_cooldowns: [f32; 16],
    pub valid_altar_slots: [bool; 25],
}

#[derive(Clone, Debug)]
pub struct DirectorDecision {
    pub target_action: usize, // 0: Пас, 1..16: Отряд, 17: Ярость, 18: Панцирь, 19: Чумные споры
    pub should_build_altar: bool,
    pub altar_target_pos: macroquad::prelude::Vec2,
    pub altar_tier: u8,
    pub spawn_sector: usize, // 0..7 — сектор появления отряда вокруг Босса
}

/// Три головы Директора: экономика, алтарь и сектор спавна.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorBrainNet {
    pub input_size: usize,
    pub hidden1_size: usize,
    pub hidden2_size: usize,

    // Слой 1: 177 -> 64
    pub w1: Vec<Vec<f32>>,
    pub b1: Vec<f32>,

    // Слой 2: 64 -> 48
    pub w2: Vec<Vec<f32>>,
    pub b2: Vec<f32>,

    // Голова 1: Единая Экономика (48 -> 20)
    pub w_econ: Vec<Vec<f32>>,
    pub b_econ: Vec<f32>,

    // Голова 2: Алтарь Крови (48 -> 3)
    pub w_altar: Vec<Vec<f32>>,
    pub b_altar: Vec<f32>,

    // Голова 3: Сектор спавна армии (48 -> 8)
    pub w_spawn: Vec<Vec<f32>>,
    pub b_spawn: Vec<f32>,
}

impl DirectorBrainNet {
    pub fn new() -> Self {
        let scale1 = (2.0 / DIRECTOR_INPUT_SIZE as f32).sqrt();
        let scale2 = (2.0 / DIRECTOR_HIDDEN1_SIZE as f32).sqrt();
        let scale_econ = (2.0 / DIRECTOR_HIDDEN2_SIZE as f32).sqrt();
        let scale_altar = (2.0 / DIRECTOR_HIDDEN2_SIZE as f32).sqrt();
        let scale_spawn = (2.0 / DIRECTOR_HIDDEN2_SIZE as f32).sqrt();

        let w1 = (0..DIRECTOR_HIDDEN1_SIZE)
            .map(|_| (0..DIRECTOR_INPUT_SIZE).map(|_| rand::gen_range(-scale1, scale1)).collect())
            .collect();
        let b1 = vec![0.0; DIRECTOR_HIDDEN1_SIZE];

        let w2 = (0..DIRECTOR_HIDDEN2_SIZE)
            .map(|_| (0..DIRECTOR_HIDDEN1_SIZE).map(|_| rand::gen_range(-scale2, scale2)).collect())
            .collect();
        let b2 = vec![0.0; DIRECTOR_HIDDEN2_SIZE];

        let w_econ = (0..DIRECTOR_ECONOMY_HEAD_SIZE)
            .map(|_| (0..DIRECTOR_HIDDEN2_SIZE).map(|_| rand::gen_range(-scale_econ, scale_econ)).collect())
            .collect();
        let b_econ = vec![0.0; DIRECTOR_ECONOMY_HEAD_SIZE];

        let w_altar = (0..DIRECTOR_ALTAR_HEAD_SIZE)
            .map(|_| (0..DIRECTOR_HIDDEN2_SIZE).map(|_| rand::gen_range(-scale_altar, scale_altar)).collect())
            .collect();
        let b_altar = vec![0.0; DIRECTOR_ALTAR_HEAD_SIZE];

        let w_spawn = (0..DIRECTOR_SPAWN_HEAD_SIZE)
            .map(|_| (0..DIRECTOR_HIDDEN2_SIZE).map(|_| rand::gen_range(-scale_spawn, scale_spawn)).collect())
            .collect();
        let b_spawn = vec![0.0; DIRECTOR_SPAWN_HEAD_SIZE];

        Self {
            input_size: DIRECTOR_INPUT_SIZE,
            hidden1_size: DIRECTOR_HIDDEN1_SIZE,
            hidden2_size: DIRECTOR_HIDDEN2_SIZE,
            w1,
            b1,
            w2,
            b2,
            w_econ,
            b_econ,
            w_altar,
            b_altar,
            w_spawn,
            b_spawn,
        }
    }

    /// Расширяет входной слой, сохраняя старые признаки и нулевые веса новых.
    pub fn upgrade_input_schema(&mut self) -> bool {
        let old_input = self.input_size;
        let old_shape_ok = old_input > 0
            && old_input <= DIRECTOR_INPUT_SIZE
            && self.hidden1_size == DIRECTOR_HIDDEN1_SIZE
            && self.hidden2_size == DIRECTOR_HIDDEN2_SIZE
            && self.w1.len() == DIRECTOR_HIDDEN1_SIZE
            && self.w1.iter().all(|r| r.len() == old_input)
            && self.b1.len() == DIRECTOR_HIDDEN1_SIZE
            && self.w2.len() == DIRECTOR_HIDDEN2_SIZE
            && self.w2.iter().all(|r| r.len() == DIRECTOR_HIDDEN1_SIZE)
            && self.b2.len() == DIRECTOR_HIDDEN2_SIZE
            && self.w_econ.len() == DIRECTOR_ECONOMY_HEAD_SIZE
            && self.w_econ.iter().all(|r| r.len() == DIRECTOR_HIDDEN2_SIZE)
            && self.b_econ.len() == DIRECTOR_ECONOMY_HEAD_SIZE
            && (self.w_altar.len() == 3 || self.w_altar.len() == DIRECTOR_ALTAR_HEAD_SIZE)
            && self.w_altar.iter().all(|r| r.len() == DIRECTOR_HIDDEN2_SIZE)
            && (self.b_altar.len() == 3 || self.b_altar.len() == DIRECTOR_ALTAR_HEAD_SIZE)
            && self.w_spawn.len() == DIRECTOR_SPAWN_HEAD_SIZE
            && self.w_spawn.iter().all(|r| r.len() == DIRECTOR_HIDDEN2_SIZE)
            && self.b_spawn.len() == DIRECTOR_SPAWN_HEAD_SIZE;
        if !old_shape_ok {
            return false;
        }

        if self.w_altar.len() == 3 {
            self.w_altar.resize_with(DIRECTOR_ALTAR_HEAD_SIZE, || vec![0.0; DIRECTOR_HIDDEN2_SIZE]);
            self.b_altar.resize(DIRECTOR_ALTAR_HEAD_SIZE, 0.0);
        }

        if old_input < DIRECTOR_INPUT_SIZE {
            for row in &mut self.w1 {
                row.resize(DIRECTOR_INPUT_SIZE, 0.0);
            }
            self.input_size = DIRECTOR_INPUT_SIZE;
        }
        self.is_compatible()
    }

    pub fn is_compatible(&self) -> bool {
        self.input_size == DIRECTOR_INPUT_SIZE
            && self.hidden1_size == DIRECTOR_HIDDEN1_SIZE
            && self.hidden2_size == DIRECTOR_HIDDEN2_SIZE
            && self.w1.len() == DIRECTOR_HIDDEN1_SIZE
            && self.w1.iter().all(|r| r.len() == DIRECTOR_INPUT_SIZE)
            && self.b1.len() == DIRECTOR_HIDDEN1_SIZE
            && self.w2.len() == DIRECTOR_HIDDEN2_SIZE
            && self.w2.iter().all(|r| r.len() == DIRECTOR_HIDDEN1_SIZE)
            && self.b2.len() == DIRECTOR_HIDDEN2_SIZE
            && self.w_econ.len() == DIRECTOR_ECONOMY_HEAD_SIZE
            && self.w_econ.iter().all(|r| r.len() == DIRECTOR_HIDDEN2_SIZE)
            && self.b_econ.len() == DIRECTOR_ECONOMY_HEAD_SIZE
            && self.w_altar.len() == DIRECTOR_ALTAR_HEAD_SIZE
            && self.w_altar.iter().all(|r| r.len() == DIRECTOR_HIDDEN2_SIZE)
            && self.b_altar.len() == DIRECTOR_ALTAR_HEAD_SIZE
            && self.w_spawn.len() == DIRECTOR_SPAWN_HEAD_SIZE
            && self.w_spawn.iter().all(|r| r.len() == DIRECTOR_HIDDEN2_SIZE)
            && self.b_spawn.len() == DIRECTOR_SPAWN_HEAD_SIZE
            && self
                .w1
                .iter()
                .chain(&self.w2)
                .chain(&self.w_econ)
                .chain(&self.w_altar)
                .chain(&self.w_spawn)
                .all(|row| row.iter().all(|w| w.is_finite()))
            && self
                .b1
                .iter()
                .chain(&self.b2)
                .chain(&self.b_econ)
                .chain(&self.b_altar)
                .chain(&self.b_spawn)
                .all(|b| b.is_finite())
    }

    /// Позиция одного из 25 слотов сетки размещения алтарей.
    pub fn altar_slot_position(slot: usize) -> macroquad::prelude::Vec2 {
        const ALTAR_SLOTS: f32 = 5.0;
        const ALTAR_MARGIN: f32 = 60.0;
        let slot = slot.min(24);
        let slot_w = (crate::core::MAP_WIDTH - ALTAR_MARGIN * 2.0) / ALTAR_SLOTS;
        let slot_h = (crate::core::MAP_HEIGHT - ALTAR_MARGIN * 2.0) / ALTAR_SLOTS;
        let sx = (slot % 5) as f32;
        let sy = (slot / 5) as f32;
        crate::content::structures::snap_altar_to_grid(macroquad::prelude::Vec2::new(
            ALTAR_MARGIN + (sx + 0.5) * slot_w,
            ALTAR_MARGIN + (sy + 0.5) * slot_h,
        ))
    }

    /// Прямой проход с маской цены, разблокировки и общего бюджета алтаря.
    pub fn forward(
        &self,
        inputs: &[f32; DIRECTOR_INPUT_SIZE],
        ctx: &DirectorActionContext,
    ) -> DirectorDecision {
        // 1. Скрытый слой 1 (SIMD zip dot product)
        let mut h1 = [0.0f32; DIRECTOR_HIDDEN1_SIZE];
        for (i, row) in self.w1.iter().enumerate() {
            let mut sum = self.b1[i];
            for (&w, &inp) in row.iter().zip(inputs.iter()) {
                sum += w * inp;
            }
            h1[i] = if sum > 0.0 { sum } else { 0.01 * sum };
        }

        // 2. Скрытый слой 2 (SIMD zip dot product)
        let mut h2 = [0.0f32; DIRECTOR_HIDDEN2_SIZE];
        for (i, row) in self.w2.iter().enumerate() {
            let mut sum = self.b2[i];
            for (&w, &h) in row.iter().zip(h1.iter()) {
                sum += w * h;
            }
            h2[i] = if sum > 0.0 { sum } else { 0.01 * sum };
        }

        // 3. Голова алтаря: выбирает ближайший доступный слот, если желаемый занят.
        let mut altar_raw = [0.0f32; DIRECTOR_ALTAR_HEAD_SIZE];
        for (i, row) in self.w_altar.iter().enumerate() {
            let mut sum = self.b_altar[i];
            for (&w, &h) in row.iter().zip(h2.iter()) {
                sum += w * h;
            }
            altar_raw[i] = 1.0 / (1.0 + (-sum).exp());
        }

        // Маскирование недоступных тиров до argmax: выбираем лучший среди готовых и доступных по очкам
        let mut best_tier = None;
        let mut best_score = f32::NEG_INFINITY;
        for i in 0..16 {
            if ctx.altar_tier_cooldowns[i] > 0.0 { continue; }
            if ctx.points < ctx.altar_tier_costs[i] { continue; }
            let score = altar_raw[3 + i];
            if score > best_score {
                best_score = score;
                best_tier = Some(i);
            }
        }
        let altar_tier_index = best_tier.unwrap_or(0);
        let altar_tier = altar_tier_index as u8 + 1;
        let altar_ready = best_tier.is_some();
        let sx = (altar_raw[1] * 5.0).floor().clamp(0.0, 4.0) as usize;
        let sy = (altar_raw[2] * 5.0).floor().clamp(0.0, 4.0) as usize;
        let requested_slot = sy * 5 + sx;
        let requested_pos = Self::altar_slot_position(requested_slot);
        let target_slot = if ctx.valid_altar_slots[requested_slot] {
            Some(requested_slot)
        } else {
            ctx.valid_altar_slots
                .iter()
                .enumerate()
                .filter(|(_, valid)| **valid)
                .min_by(|(a, _), (b, _)| {
                    let da = Self::altar_slot_position(*a).distance_squared(requested_pos);
                    let db = Self::altar_slot_position(*b).distance_squared(requested_pos);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(slot, _)| slot)
        };
        let should_build_altar = altar_raw[0] > 0.5 && altar_ready && target_slot.is_some();
        let altar_target_pos = target_slot
            .map(Self::altar_slot_position)
            .unwrap_or(requested_pos);

        // 4. Голова экономики: маска учитывает остаток после одновременной постройки.
        let points_after_altar = if should_build_altar {
            (ctx.points - ctx.altar_tier_costs[altar_tier_index]).max(0.0)
        } else {
            ctx.points
        };
        let mut best_action = 0usize;
        let mut max_econ_logit = f32::NEG_INFINITY;

        for i in 0..DIRECTOR_ECONOMY_HEAD_SIZE {
            let allowed = match i {
                0 => true,
                1..=16 => {
                    let config = crate::content::mobs::get_squad_config(i as u8);
                    ctx.total_points_spent >= config.unlock_spent_threshold
                        && points_after_altar >= config.cost
                        && ctx.squad_cooldowns[i - 1] <= 0.0
                }
                17 => ctx.grade_fury < crate::ecs::world::MAX_DIRECTOR_GRADE && points_after_altar >= 100.0 * (ctx.grade_fury as f32 + 1.0),
                18 => ctx.grade_carapace < crate::ecs::world::MAX_DIRECTOR_GRADE && points_after_altar >= 120.0 * (ctx.grade_carapace as f32 + 1.0),
                19 => ctx.grade_pestilence < crate::ecs::world::MAX_DIRECTOR_GRADE && points_after_altar >= 150.0 * (ctx.grade_pestilence as f32 + 1.0),
                _ => false,
            };
            if !allowed {
                continue;
            }

            let mut sum = self.b_econ[i];
            let row = &self.w_econ[i];
            for (&w, &h) in row.iter().zip(h2.iter()) {
                sum += w * h;
            }
            if sum > max_econ_logit {
                max_econ_logit = sum;
                best_action = i;
            }
        }

        // 5. Голова спавна: 8 секторов вокруг Босса (SIMD zip dot product)
        let mut spawn_sector = 0usize;
        let mut best_spawn_logit = f32::NEG_INFINITY;
        for (i, row) in self.w_spawn.iter().enumerate() {
            let mut sum = self.b_spawn[i];
            for (&w, &h) in row.iter().zip(h2.iter()) {
                sum += w * h;
            }
            if sum > best_spawn_logit {
                best_spawn_logit = sum;
                spawn_sector = i;
            }
        }

        DirectorDecision {
            target_action: best_action,
            should_build_altar,
            altar_target_pos,
            altar_tier,
            spawn_sector,
        }
    }

    pub fn mutate(&mut self, rate: f32, sigma: f32) {
        fn mutate_layer(weights: &mut [Vec<f32>], biases: &mut [f32], rate: f32, sigma: f32) {
            for row in weights.iter_mut() {
                for w in row.iter_mut() {
                    if rand::gen_range(0.0, 1.0) < rate {
                        *w += rand::gen_range(-sigma, sigma);
                    }
                }
            }
            for b in biases.iter_mut() {
                if rand::gen_range(0.0, 1.0) < rate {
                    *b += rand::gen_range(-sigma, sigma);
                }
            }
        }

        let s1 = sigma * (48.0 / DIRECTOR_INPUT_SIZE as f32).sqrt();
        let s2 = sigma * (48.0 / DIRECTOR_HIDDEN1_SIZE as f32).sqrt();
        mutate_layer(&mut self.w1, &mut self.b1, rate, s1);
        mutate_layer(&mut self.w2, &mut self.b2, rate, s2);
        mutate_layer(&mut self.w_econ, &mut self.b_econ, rate, sigma);
        mutate_layer(&mut self.w_altar, &mut self.b_altar, rate, sigma);
        mutate_layer(&mut self.w_spawn, &mut self.b_spawn, rate, sigma);
    }

    pub fn decay(&mut self, factor: f32) {
        let k = (1.0 - factor).clamp(0.0, 1.0);
        for layer in [
            &mut self.w1,
            &mut self.w2,
            &mut self.w_econ,
            &mut self.w_altar,
            &mut self.w_spawn,
        ] {
            for row in layer.iter_mut() {
                for w in row.iter_mut() {
                    if w.abs() > 1.5 {
                        *w *= k;
                    }
                }
            }
        }
    }

    pub fn crossover(&self, other: &Self) -> Self {
        fn crossover_layer(
            w_out: &mut [Vec<f32>],
            b_out: &mut [f32],
            w_other: &[Vec<f32>],
            b_other: &[f32],
        ) {
            for i in 0..w_out.len() {
                for j in 0..w_out[i].len() {
                    if rand::gen_range(0.0, 1.0) < 0.5 {
                        w_out[i][j] = w_other[i][j];
                    }
                }
                if rand::gen_range(0.0, 1.0) < 0.5 {
                    b_out[i] = b_other[i];
                }
            }
        }

        let mut child = self.clone();
        crossover_layer(&mut child.w1, &mut child.b1, &other.w1, &other.b1);
        crossover_layer(&mut child.w2, &mut child.b2, &other.w2, &other.b2);
        crossover_layer(&mut child.w_econ, &mut child.b_econ, &other.w_econ, &other.b_econ);
        crossover_layer(&mut child.w_altar, &mut child.b_altar, &other.w_altar, &other.b_altar);
        crossover_layer(&mut child.w_spawn, &mut child.b_spawn, &other.w_spawn, &other.b_spawn);
        child
    }

    pub fn total_synapses(&self) -> usize {
        (DIRECTOR_INPUT_SIZE * DIRECTOR_HIDDEN1_SIZE + DIRECTOR_HIDDEN1_SIZE)
            + (DIRECTOR_HIDDEN1_SIZE * DIRECTOR_HIDDEN2_SIZE + DIRECTOR_HIDDEN2_SIZE)
            + (DIRECTOR_HIDDEN2_SIZE * DIRECTOR_ECONOMY_HEAD_SIZE + DIRECTOR_ECONOMY_HEAD_SIZE)
            + (DIRECTOR_HIDDEN2_SIZE * DIRECTOR_ALTAR_HEAD_SIZE + DIRECTOR_ALTAR_HEAD_SIZE)
            + (DIRECTOR_HIDDEN2_SIZE * DIRECTOR_SPAWN_HEAD_SIZE + DIRECTOR_SPAWN_HEAD_SIZE)
    }
}
