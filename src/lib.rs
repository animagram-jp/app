pub mod dice;
pub mod table;
pub mod app;
pub mod character;
pub mod list;
pub mod js_client;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang { En, Ja }

pub fn n_d_n(count: u32, sides: u32) -> u32 {
    use rand::RngExt;
    (0..count).map(|_| rand::rng().random_range(1..=sides)).sum()
}
