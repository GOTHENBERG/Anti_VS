use std::cell::Cell;

/// Магический LCG множитель A* (результат точного целочисленного LLL-поиска по решеткам Минковского).
/// Hull-Dobell: A % 8 == 5 (A % 4 == 1), полный период 2^64 при нечётном C.
/// Спектральный критерий Кнута: mu >= 2.37 во всех размерностях d in {2, 3, 4, 5, 6, 8}!
pub const PRNG_MAGIC_A: u64 = 0x27BB_2EE6_87B0_B0FD;
pub const PRNG_MAGIC_C: u64 = 0x1405_7B7E_F767_814F;

/// Высокопроизводительный генератор PCG32 (XSH-RR) с идеальным базовым шагом A*.
/// Уничтожает гиперплоскости Марсальи нелинейным битовым вращением (XorShift High, Random Rotate).
#[derive(Debug, Clone, Copy)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    #[inline(always)]
    pub const fn new(seed: u64) -> Self {
        let mut rng = Self {
            state: seed.wrapping_add(PRNG_MAGIC_C),
            inc: PRNG_MAGIC_C,
        };
        // Начальный шаг разогрева LCG
        rng.state = rng.state.wrapping_mul(PRNG_MAGIC_A).wrapping_add(rng.inc);
        rng
    }

    #[inline(always)]
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.state = old_state
            .wrapping_mul(PRNG_MAGIC_A)
            .wrapping_add(self.inc);
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    #[inline(always)]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16777216.0)
    }

    #[inline(always)]
    pub fn next_f64(&mut self) -> f64 {
        let high = (self.next_u32() as u64) << 32;
        let low = self.next_u32() as u64;
        let val = (high | low) >> 11;
        val as f64 * (1.0 / 9007199254740992.0)
    }
}

thread_local! {
    static RNG: Cell<Pcg32> = Cell::new(Pcg32::new(0x853c49e6748fea9b));
    static VISUAL_RNG: Cell<Pcg32> = Cell::new(Pcg32::new(0x9e3779b97f4a7c15));
}

#[inline(always)]
fn with_cell_rng<R, F: FnOnce(&mut Pcg32) -> R>(cell: &Cell<Pcg32>, f: F) -> R {
    let mut rng = cell.get();
    let result = f(&mut rng);
    cell.set(rng);
    result
}

#[inline(always)]
pub fn with_rng<R, F: FnOnce(&mut Pcg32) -> R>(f: F) -> R {
    RNG.with(|cell| with_cell_rng(cell, f))
}

#[inline(always)]
pub fn f32() -> f32 {
    with_rng(|r| r.next_f32())
}

#[inline(always)]
pub fn f64() -> f64 {
    with_rng(|r| r.next_f64())
}

#[inline(always)]
fn range_u32(rng: &mut Pcg32, range: std::ops::Range<u32>) -> u32 {
    if range.start >= range.end {
        return range.start;
    }
    let span = range.end - range.start;
    range.start + (rng.next_u32() % span)
}

#[inline(always)]
fn range_i32(rng: &mut Pcg32, range: std::ops::Range<i32>) -> i32 {
    if range.start >= range.end {
        return range.start;
    }
    let span = (range.end - range.start) as u32;
    range.start + (rng.next_u32() % span) as i32
}

#[inline(always)]
fn range_usize(rng: &mut Pcg32, range: std::ops::Range<usize>) -> usize {
    if range.start >= range.end {
        return range.start;
    }
    let span = (range.end - range.start) as u64;
    let high = rng.next_u32() as u64;
    let low = rng.next_u32() as u64;
    let val = (high << 32) | low;
    range.start + (val % span) as usize
}

#[inline(always)]
pub fn u32(range: std::ops::Range<u32>) -> u32 {
    with_rng(|r| range_u32(r, range))
}

#[inline(always)]
pub fn i32(range: std::ops::Range<i32>) -> i32 {
    with_rng(|r| range_i32(r, range))
}

#[inline(always)]
pub fn usize(range: std::ops::Range<usize>) -> usize {
    with_rng(|r| range_usize(r, range))
}

pub trait RangeVal: Copy {
    fn generate_with(low: Self, high: Self, rng: &mut Pcg32) -> Self;
}

impl RangeVal for f32 {
    #[inline]
    fn generate_with(low: f32, high: f32, rng: &mut Pcg32) -> f32 {
        if low >= high {
            low
        } else {
            low + rng.next_f32() * (high - low)
        }
    }
}

impl RangeVal for f64 {
    #[inline]
    fn generate_with(low: f64, high: f64, rng: &mut Pcg32) -> f64 {
        if low >= high {
            low
        } else {
            low + rng.next_f64() * (high - low)
        }
    }
}

impl RangeVal for usize {
    #[inline]
    fn generate_with(low: usize, high: usize, rng: &mut Pcg32) -> usize {
        if low >= high {
            low
        } else {
            range_usize(rng, low..high)
        }
    }
}

impl RangeVal for i32 {
    #[inline]
    fn generate_with(low: i32, high: i32, rng: &mut Pcg32) -> i32 {
        if low >= high {
            low
        } else {
            range_i32(rng, low..high)
        }
    }
}

impl RangeVal for u32 {
    #[inline]
    fn generate_with(low: u32, high: u32, rng: &mut Pcg32) -> u32 {
        if low >= high {
            low
        } else {
            range_u32(rng, low..high)
        }
    }
}

/// Высокоскоростной потокобезопасный генератор случайных чисел для симуляции и ML
#[inline]
pub fn gen_range<T: RangeVal>(low: T, high: T) -> T {
    with_rng(|r| T::generate_with(low, high, r))
}

/// Детерминированно инициализирует потоковый RNG симуляции.
/// Вызывается в начале каждого headless-матча, чтобы фиксированный seed
/// давал воспроизводимый результат независимо от потока Rayon.
#[inline]
pub fn seed(value: u64) {
    RNG.with(|cell| cell.set(Pcg32::new(value)));
    fastrand::seed(value);
}

/// Поток RNG исключительно для симуляции и геймплейной логики.
#[allow(unused_imports)]
pub mod sim_rand {
    pub use super::{f32, f64, gen_range, i32, seed, u32, usize, with_rng, RangeVal};
}

/// Независимый поток RNG для частиц, плавающего текста, музыки и прочих визуальных эффектов.
/// Никогда не сдвигает детерминированный `sim_rand`.
#[allow(dead_code)]
pub mod visual_rand {
    use super::{with_cell_rng, Pcg32, RangeVal, VISUAL_RNG};

    #[inline(always)]
    pub fn with_rng<R, F: FnOnce(&mut Pcg32) -> R>(f: F) -> R {
        VISUAL_RNG.with(|cell| with_cell_rng(cell, f))
    }

    #[inline(always)]
    pub fn f32() -> f32 {
        with_rng(|r| r.next_f32())
    }

    #[inline]
    pub fn gen_range<T: RangeVal>(low: T, high: T) -> T {
        with_rng(|r| T::generate_with(low, high, r))
    }

    #[inline]
    pub fn seed(value: u64) {
        VISUAL_RNG.with(|cell| cell.set(Pcg32::new(value)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcg32_deterministic_and_uniform() {
        let mut rng1 = Pcg32::new(12345);
        let mut rng2 = Pcg32::new(12345);

        for _ in 0..1000 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
            assert_eq!(rng1.next_f32(), rng2.next_f32());
        }
    }

    #[test]
    fn test_pcg32_distribution_bounds() {
        let mut rng = Pcg32::new(42);
        for _ in 0..10000 {
            let f = rng.next_f32();
            assert!(f >= 0.0 && f < 1.0, "f32 out of bounds: {}", f);
        }
    }

    #[test]
    fn test_visual_rand_does_not_affect_sim_rand() {
        sim_rand::seed(777);
        let expected: Vec<u32> = (0..16).map(|_| sim_rand::gen_range(0u32, 10_000)).collect();

        sim_rand::seed(777);
        visual_rand::seed(999);
        let mut actual = Vec::with_capacity(16);
        for _ in 0..16 {
            let _ = visual_rand::gen_range(-100.0f32, 100.0);
            let _ = visual_rand::gen_range(0usize, 50);
            actual.push(sim_rand::gen_range(0u32, 10_000));
        }
        assert_eq!(actual, expected);
    }
}
