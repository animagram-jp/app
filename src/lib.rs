pub mod list;
pub mod character;
pub mod dice;
pub mod table;
pub mod js_client;
pub mod app;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang { En, Ja }

// ============================================================
// Roll
// ============================================================

pub fn n_d_n(count: u32, sides: u32) -> u32 {
    use rand::RngExt;
    (0..count).map(|_| rand::rng().random_range(1..=sides)).sum()
}
