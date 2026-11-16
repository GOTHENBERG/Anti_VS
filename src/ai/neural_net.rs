use crate::core::rand;
use serde::{Deserialize, Serialize};

pub const INPUT_SIZE: usize = 48;
pub const HIDDEN1_SIZE: usize = 32;
pub const HIDDEN2_SIZE: usize = 24;
pub const OUTPUT_SIZE: usize = 8;

/// Полносвязная глубокая нейронная сеть (Deep MLP)
/// Архитектура: 48 сенсорных входов -> Скрытый слой 32 (ReLU) -> Скрытый слой 24 (ReLU) -> 8 выходов (Tanh/Sigmoid)
/// Всего синапсов (весов и смещений): 48*32 + 32 + 32*24 + 24 + 24*8 + 8 = 2,560 параметров!
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuralNet {
    pub input_size: usize,
    pub hidden1_size: usize,
    pub hidden2_size: usize,
    pub output_size: usize,

    // Слой 1: 48 -> 32
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
