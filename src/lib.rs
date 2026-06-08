// #![no_std]
extern crate core;
extern crate alloc;

use rand::{rng, RngExt};

pub mod list;
pub mod timestamp;
pub mod temporal;
pub mod js_client;
pub mod store;
pub mod data_struct;
// pub mod app;
pub mod character;
pub mod roll;
pub mod event;

// pub mod temporal;
// pub mod calendar;
// pub mod upx;


// ============================================================
// Lang
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang { En, Ja }

impl Lang {
    fn label(self) -> &'static str {
        match self {
            Self::En => "en-US",
            Self::Ja => "ja",
        }
    }
}

// ============================================================
// Roll
// ============================================================

pub fn n_d_n(count: u32, sides: u32) -> u32 {
    (0..count).map(|_| rng().random_range(1..=sides)).sum()
}

// ============================================================
// Percent Roll (1d100 + Bonus/Penalty Dice)
// ============================================================

pub fn percent_roll(bonus: i32) -> (u32, Vec<u32>) {
    let mut rng = rng();
    let roll_tens = |r: &mut _| {
        let d: u32 = RngExt::random_range(r, 1..=10u32);
        if d == 10 { 0 } else { d * 10 }
    };
    let ones: u32 = {
        let d: u32 = rng.random_range(1..=10u32);
        if d == 10 { 0 } else { d }
    };
    let count = (bonus.unsigned_abs() + 1) as usize;
    let tens_list: Vec<u32> = (0..count).map(|_| roll_tens(&mut rng)).collect();
    let dice_list: Vec<u32> = tens_list
        .iter()
        .map(|&t| { let v = t + ones; if v == 0 { 100 } else { v } })
        .collect();
    let total = if bonus >= 0 {
        *dice_list.iter().min().unwrap()
    } else {
        *dice_list.iter().max().unwrap()
    };
    (total, dice_list)
}
