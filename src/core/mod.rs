pub mod blue_noise;
pub mod crowd;
pub mod math;
pub mod random;
pub mod spatial_grid;
pub mod time;

pub use random as rand;
#[allow(unused_imports)]
pub use random::sim_rand;
pub use random::visual_rand;

pub const MAP_WIDTH: f32 = 25920.0;
pub const MAP_HEIGHT: f32 = 17280.0;
pub const MAP_DIAGONAL: f32 = 31151.96;
