pub mod handler;
pub mod import;
pub mod investigator;
pub mod page;
pub mod roll;

/// (count, sides, modifier)
pub type Dice = (i8, u8, i8);

pub mod dice {
    use alloc::{
        format,
        string::{String, ToString},
    };

    use rand::{RngExt as _, rand_core::UnwrapErr, rngs::SysRng};

    use super::Dice;

    pub fn display(dice: &[Dice]) -> String {
        let s = dice
            .iter()
            .map(|&(count, sides, modifier)| {
                let dice_str = if count == 0 || sides == 0 {
                    String::new()
                } else {
                    format!("{count}D{sides}")
                };
                let modifier_str = match modifier {
                    0 => String::new(),
                    m if m > 0 => format!("+{m}"),
                    m => format!("{m}"),
                };
                format!("{dice_str}{modifier_str}")
            })
            .collect::<String>();
        s.trim_start_matches('+').to_string()
    }
    pub fn roll(dice: &[Dice]) -> i32 {
        dice.iter()
            .map(|&(count, sides, modifier)| {
                let rolled = if count != 0 && sides > 0 {
                    let mut rng = UnwrapErr(SysRng);
                    let sum: i32 =
                        (0..count.unsigned_abs()).map(|_| rng.random_range(1..=sides as i32)).sum();
                    if count < 0 { -sum } else { sum }
                } else {
                    0
                };
                rolled + modifier as i32
            })
            .sum()
    }
    pub fn percent_roll(level: i8) -> u8 {
        let mut rng = UnwrapErr(SysRng);
        let ones: u8 = rng.random_range(0..=9u8);
        let offset: u8 = if ones == 0 { 1 } else { 0 };
        let rolls_count = 1 + level.unsigned_abs() as usize;
        let tens = if level > 0 {
            (0..rolls_count).map(|_| rng.random_range(0..=9u8) + offset).min().unwrap()
        } else if level < 0 {
            (0..rolls_count).map(|_| rng.random_range(0..=9u8) + offset).max().unwrap()
        } else {
            rng.random_range(0..=9u8) + offset
        };
        tens * 10 + ones
    }
}

crate::enumerate! {
    pub Kind(u8) {
        Other,
        Investigator,
    }
}
