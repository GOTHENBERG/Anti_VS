//! 256x256 Toroidal Blue Noise Rank-Order Texture.
//!
//! Precomputed rank texture with uniform distribution (each value in 0..255
//! appears exactly 256 times) and blue noise high-pass spectral characteristics.
//! Used for instant O(1) Poisson-disk-like sampling and cluster-free mob/particle spawning.

pub const BLUE_NOISE_DIM: usize = 256;
pub const BLUE_NOISE_SIZE: usize = BLUE_NOISE_DIM * BLUE_NOISE_DIM; // 65536

pub static BLUE_NOISE_256: &[u8; BLUE_NOISE_SIZE] = include_bytes!("../../data/blue_noise_256.bin");

/// Sample the toroidal blue noise rank texture at discrete integer coordinates (x, y).
#[inline(always)]
pub fn sample_blue_noise(x: i32, y: i32) -> u8 {
    let ux = (x & 255) as usize;
    let uy = (y & 255) as usize;
    BLUE_NOISE_256[uy * BLUE_NOISE_DIM + ux]
}

/// Normalized blue noise sample in [0.0, 1.0).
#[inline(always)]
pub fn sample_blue_noise_f32(x: i32, y: i32) -> f32 {
    sample_blue_noise(x, y) as f32 / 256.0
}

/// Instant O(1) Poisson-like acceptance test: returns true if a candidate at (x, y)
/// with given density threshold [0.0..1.0] should be kept.
#[inline(always)]
pub fn should_spawn_candidate(x: i32, y: i32, density: f32) -> bool {
    if !(density > 0.0) {
        return false;
    }
    if density >= 1.0 {
        return true;
    }
    let cutoff = (density * 256.0) as u16;
    (sample_blue_noise(x, y) as u16) < cutoff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blue_noise_table_loaded() {
        assert_eq!(BLUE_NOISE_256.len(), 65536);
        let mut counts = [0usize; 256];
        for &val in BLUE_NOISE_256.iter() {
            counts[val as usize] += 1;
        }
        // Every byte 0..255 must appear exactly 256 times
        for (i, &count) in counts.iter().enumerate() {
            assert_eq!(count, 256, "Byte {} count is {}, expected 256", i, count);
        }
    }

    #[test]
    fn test_toroidal_sampling() {
        let v0 = sample_blue_noise(0, 0);
        let v_wrap = sample_blue_noise(256, 256);
        let v_neg = sample_blue_noise(-256, -256);
        assert_eq!(v0, v_wrap);
        assert_eq!(v0, v_neg);
    }

    #[test]
    fn test_blue_noise_density_thresholds_exact() {
        let mut kept_zero = 0usize;
        let mut kept_neg = 0usize;
        let mut kept_nan = 0usize;
        let mut kept_one = 0usize;
        let mut kept_over = 0usize;
        let mut kept_half = 0usize;
        let mut kept_quarter = 0usize;

        for y in 0..256 {
            for x in 0..256 {
                if should_spawn_candidate(x, y, 0.0) { kept_zero += 1; }
                if should_spawn_candidate(x, y, -0.5) { kept_neg += 1; }
                if should_spawn_candidate(x, y, f32::NAN) { kept_nan += 1; }
                if should_spawn_candidate(x, y, 1.0) { kept_one += 1; }
                if should_spawn_candidate(x, y, 1.5) { kept_over += 1; }
                if should_spawn_candidate(x, y, 0.5) { kept_half += 1; }
                if should_spawn_candidate(x, y, 0.25) { kept_quarter += 1; }
            }
        }

        assert_eq!(kept_zero, 0, "density 0.0 must reject every sample");
        assert_eq!(kept_neg, 0, "negative density must reject every sample");
        assert_eq!(kept_nan, 0, "NaN density must reject every sample");
        assert_eq!(kept_one, BLUE_NOISE_SIZE, "density 1.0 must accept all 65536 samples");
        assert_eq!(kept_over, BLUE_NOISE_SIZE, "density > 1.0 must accept all 65536 samples");
        assert_eq!(kept_half, BLUE_NOISE_SIZE / 2);
        assert_eq!(kept_quarter, BLUE_NOISE_SIZE / 4);
    }
}
