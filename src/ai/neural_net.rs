use crate::core::rand;
use serde::{Deserialize, Serialize};

pub const INPUT_SIZE: usize = 150;
pub const HIDDEN1_SIZE: usize = 32;
pub const HIDDEN2_SIZE: usize = 24;
pub const OUTPUT_SIZE: usize = 8;

/// Полносвязная глубокая нейронная сеть (Deep MLP)
/// Архитектура: 150 сенсорных входов -> Скрытый слой 32 (ReLU) -> Скрытый слой 24 (ReLU) -> 8 выходов (Tanh/Sigmoid)
/// Всего синапсов (весов и смещений): 150*32 + 32 + 32*24 + 24 + 24*8 + 8 = 5,824 параметров!
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuralNet {
    pub input_size: usize,
    pub hidden1_size: usize,
    pub hidden2_size: usize,
    pub output_size: usize,

    // Слой 1: 150 -> 32
    pub w1: Vec<Vec<f32>>,
    pub b1: Vec<f32>,

    // Слой 2: 32 -> 24
    pub w2: Vec<Vec<f32>>,
    pub b2: Vec<f32>,

    // Слой 3: 24 -> 8
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

    /// Прямой проход нейросети (Forward inference pass)
    /// Выполняется за микросекунды без аллокаций памяти!
    pub fn forward(&self, inputs: &[f32; INPUT_SIZE]) -> [f32; OUTPUT_SIZE] {
        // 1. Скрытый слой 1 (Linear + ReLU)
        let mut h1 = [0.0f32; HIDDEN1_SIZE];
        for i in 0..HIDDEN1_SIZE {
            let mut sum = self.b1[i];
            let row = &self.w1[i];
            for j in 0..INPUT_SIZE {
                sum += row[j] * inputs[j];
            }
            h1[i] = if sum > 0.0 { sum } else { 0.01 * sum }; // Leaky ReLU
        }

        // 2. Скрытый слой 2 (Linear + ReLU)
        let mut h2 = [0.0f32; HIDDEN2_SIZE];
        for i in 0..HIDDEN2_SIZE {
            let mut sum = self.b2[i];
            let row = &self.w2[i];
            for j in 0..HIDDEN1_SIZE {
                sum += row[j] * h1[j];
            }
            h2[i] = if sum > 0.0 { sum } else { 0.01 * sum };
        }

        // 3. Выходной слой (Linear + Non-linearities)
        let mut out = [0.0f32; OUTPUT_SIZE];
        for i in 0..OUTPUT_SIZE {
            let mut sum = self.b3[i];
            let row = &self.w3[i];
            for j in 0..HIDDEN2_SIZE {
                sum += row[j] * h2[j];
            }
            // Выходы 0 и 1: Направление движения (-1.0 .. 1.0 через Tanh)
            if i < 2 {
                out[i] = sum.tanh();
            } else {
                // Выходы 2..7: Множители поведения (Sigmoid 0.0 .. 1.0)
                out[i] = 1.0 / (1.0 + (-sum).exp());
            }
        }

        out
    }

    /// Мутация синаптических весов для эволюции
    pub fn mutate(&mut self, rate: f32, sigma: f32) {
        for row in self.w1.iter_mut() {
            for w in row.iter_mut() {
                if rand::gen_range(0.0, 1.0) < rate {
                    *w += rand::gen_range(-sigma, sigma);
                }
            }
        }
        for b in self.b1.iter_mut() {
            if rand::gen_range(0.0, 1.0) < rate {
                *b += rand::gen_range(-sigma, sigma);
            }
        }

        for row in self.w2.iter_mut() {
            for w in row.iter_mut() {
                if rand::gen_range(0.0, 1.0) < rate {
                    *w += rand::gen_range(-sigma, sigma);
                }
            }
        }
        for b in self.b2.iter_mut() {
            if rand::gen_range(0.0, 1.0) < rate {
                *b += rand::gen_range(-sigma, sigma);
            }
        }

        for row in self.w3.iter_mut() {
            for w in row.iter_mut() {
                if rand::gen_range(0.0, 1.0) < rate {
                    *w += rand::gen_range(-sigma, sigma);
                }
            }
        }
        for b in self.b3.iter_mut() {
            if rand::gen_range(0.0, 1.0) < rate {
                *b += rand::gen_range(-sigma, sigma);
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

pub const DIRECTOR_INPUT_SIZE: usize = 132;
pub const DIRECTOR_HIDDEN1_SIZE: usize = 64;
pub const DIRECTOR_HIDDEN2_SIZE: usize = 48;
pub const DIRECTOR_ECONOMY_HEAD_SIZE: usize = 20; // 0: Пас/Копить, 1..16: Тиры 1..16, 17: Ярость, 18: Панцирь, 19: Чумные споры
pub const DIRECTOR_ALTAR_HEAD_SIZE: usize = 3;   // [0]: Триггер постройки, [1]: X, [2]: Y

#[derive(Clone, Debug)]
pub struct DirectorDecision {
    pub target_action: usize,     // 0: Пас, 1..16: Отряд 1..16, 17: Ярость, 18: Панцирь, 19: Чумные споры
    pub should_build_altar: bool,
    pub altar_target_pos: macroquad::prelude::Vec2,
}

/// Двухголовая глубокая нейросеть Директора (Dual-Head RTS Commander Brain)
/// Архитектура: 132 сенсорных входа -> 64 (ReLU) -> 48 (ReLU) -> 2 головы решений:
/// 1. Голова Экономики (20 выходов): Единая цель бюджета (Пас / 16 Тиров мобов / 3 Грейда армии)
/// 2. Голова Алтаря (3 выхода): Постройка и пространственные координаты (X, Y) Алтаря Крови
/// Всего синапсов: 12,759 параметров!
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectorBrainNet {
    pub input_size: usize,
    pub hidden1_size: usize,
    pub hidden2_size: usize,

    // Слой 1: 132 -> 64
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
}

impl DirectorBrainNet {
    pub fn new() -> Self {
        let scale1 = (2.0 / DIRECTOR_INPUT_SIZE as f32).sqrt();
        let scale2 = (2.0 / DIRECTOR_HIDDEN1_SIZE as f32).sqrt();
        let scale_econ = (2.0 / DIRECTOR_HIDDEN2_SIZE as f32).sqrt();
        let scale_altar = (2.0 / DIRECTOR_HIDDEN2_SIZE as f32).sqrt();

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
        }
    }

    /// Прямой проход нейросети Директора (Forward Pass с Action Masking)
    pub fn forward(&self, inputs: &[f32; DIRECTOR_INPUT_SIZE], total_points_spent: f32) -> DirectorDecision {
        // 1. Скрытый слой 1 (Linear + Leaky ReLU)
        let mut h1 = [0.0f32; DIRECTOR_HIDDEN1_SIZE];
        for i in 0..DIRECTOR_HIDDEN1_SIZE {
            let mut sum = self.b1[i];
            let row = &self.w1[i];
            for j in 0..DIRECTOR_INPUT_SIZE {
                sum += row[j] * inputs[j];
            }
            h1[i] = if sum > 0.0 { sum } else { 0.01 * sum };
        }

        // 2. Скрытый слой 2 (Linear + Leaky ReLU)
        let mut h2 = [0.0f32; DIRECTOR_HIDDEN2_SIZE];
        for i in 0..DIRECTOR_HIDDEN2_SIZE {
            let mut sum = self.b2[i];
            let row = &self.w2[i];
            for j in 0..DIRECTOR_HIDDEN1_SIZE {
                sum += row[j] * h1[j];
            }
            h2[i] = if sum > 0.0 { sum } else { 0.01 * sum };
        }

        // 3. Голова 1: Единая Экономика (20 логитов -> argmax с Action Masking)
        // Заблокированные тиры отрядов не могут быть выбраны целью (Action Masking)
        let mut best_action = 0;
        let mut max_econ_logit = f32::NEG_INFINITY;
        for i in 0..DIRECTOR_ECONOMY_HEAD_SIZE {
            if (1..=16).contains(&i) {
                let tier = i as u8;
                let threshold = crate::content::mobs::get_squad_config(tier).unlock_spent_threshold;
                if total_points_spent < threshold {
                    continue; // Заблокировано в дереве технологий
                }
            }

            let mut sum = self.b_econ[i];
            let row = &self.w_econ[i];
            for j in 0..DIRECTOR_HIDDEN2_SIZE {
                sum += row[j] * h2[j];
            }
            if sum > max_econ_logit {
                max_econ_logit = sum;
                best_action = i;
            }
        }

        // 4. Голова 2: Алтарь Крови (3 выхода)
        // [0]: Триггер (Sigmoid > 0.5)
        // [1]: X координата (Sigmoid -> [35.0, MAP_WIDTH - 35.0])
        // [2]: Y координата (Sigmoid -> [35.0, MAP_HEIGHT - 35.0])
        let mut altar_raw = [0.0f32; DIRECTOR_ALTAR_HEAD_SIZE];
        for i in 0..DIRECTOR_ALTAR_HEAD_SIZE {
            let mut sum = self.b_altar[i];
            let row = &self.w_altar[i];
            for j in 0..DIRECTOR_HIDDEN2_SIZE {
                sum += row[j] * h2[j];
            }
            altar_raw[i] = 1.0 / (1.0 + (-sum).exp());
        }

        let should_build_altar = altar_raw[0] > 0.5;
        let altar_x = 35.0 + altar_raw[1] * (crate::core::MAP_WIDTH - 70.0);
        let altar_y = 35.0 + altar_raw[2] * (crate::core::MAP_HEIGHT - 70.0);

        DirectorDecision {
            target_action: best_action,
            should_build_altar,
            altar_target_pos: macroquad::prelude::Vec2::new(altar_x, altar_y),
        }
    }

    /// Мутация синаптических весов для эволюции Директора
    pub fn mutate(&mut self, rate: f32, sigma: f32) {
        fn mutate_layer(weights: &mut Vec<Vec<f32>>, biases: &mut Vec<f32>, rate: f32, sigma: f32) {
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

        mutate_layer(&mut self.w1, &mut self.b1, rate, sigma);
        mutate_layer(&mut self.w2, &mut self.b2, rate, sigma);
        mutate_layer(&mut self.w_econ, &mut self.b_econ, rate, sigma);
        mutate_layer(&mut self.w_altar, &mut self.b_altar, rate, sigma);
    }

    /// Генетический кроссовер весовых матриц двух нейросетей Директора
    pub fn crossover(&self, other: &Self) -> Self {
        fn crossover_layer(
            w_out: &mut Vec<Vec<f32>>,
            b_out: &mut Vec<f32>,
            w_other: &Vec<Vec<f32>>,
            b_other: &Vec<f32>,
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
        child
    }

    /// Общее число параметров (синапсов) в мозге Директора
    pub fn total_synapses(&self) -> usize {
        (DIRECTOR_INPUT_SIZE * DIRECTOR_HIDDEN1_SIZE + DIRECTOR_HIDDEN1_SIZE)
            + (DIRECTOR_HIDDEN1_SIZE * DIRECTOR_HIDDEN2_SIZE + DIRECTOR_HIDDEN2_SIZE)
            + (DIRECTOR_HIDDEN2_SIZE * DIRECTOR_ECONOMY_HEAD_SIZE + DIRECTOR_ECONOMY_HEAD_SIZE)
            + (DIRECTOR_HIDDEN2_SIZE * DIRECTOR_ALTAR_HEAD_SIZE + DIRECTOR_ALTAR_HEAD_SIZE)
    }
}
