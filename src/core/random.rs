pub trait RangeVal: Copy {
    fn generate(low: Self, high: Self) -> Self;
}

impl RangeVal for f32 {
    #[inline]
    fn generate(low: f32, high: f32) -> f32 {
        if low >= high {
            low
        } else {
            low + fastrand::f32() * (high - low)
        }
    }
}

impl RangeVal for f64 {
    #[inline]
    fn generate(low: f64, high: f64) -> f64 {
        if low >= high {
            low
        } else {
            low + fastrand::f64() * (high - low)
        }
    }
}

impl RangeVal for usize {
    #[inline]
    fn generate(low: usize, high: usize) -> usize {
        if low >= high {
            low
        } else {
            fastrand::usize(low..high)
        }
    }
}

impl RangeVal for i32 {
    #[inline]
    fn generate(low: i32, high: i32) -> i32 {
        if low >= high {
            low
        } else {
            fastrand::i32(low..high)
        }
    }
}

impl RangeVal for u32 {
    #[inline]
    fn generate(low: u32, high: u32) -> u32 {
        if low >= high {
            low
        } else {
            fastrand::u32(low..high)
        }
    }
}

/// Высокоскоростной потокобезопасный генератор случайных чисел для симуляции и ML
#[inline]
pub fn gen_range<T: RangeVal>(low: T, high: T) -> T {
    T::generate(low, high)
}

/// Детерминированно инициализирует потоковый RNG.
/// Вызывается в начале каждого headless-матча, чтобы фиксированный seed
/// давал воспроизводимый результат независимо от потока Rayon.
#[inline]
pub fn seed(value: u64) {
    fastrand::seed(value);
}
