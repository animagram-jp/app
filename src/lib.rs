#![no_std]
#![feature(adt_const_params)]
#![cfg_attr(feature = "coc7th", feature(const_param_ty_trait))]
#![cfg_attr(feature = "coc7th", feature(variant_count))]
#![cfg_attr(
    all(target_arch = "wasm32", target_feature = "atomics"),
    feature(stdarch_wasm_atomic_wait)
)]

extern crate alloc;
extern crate core;
#[cfg(all(
    target_arch = "wasm32",
    feature = "file_store",
    not(target_feature = "atomics"),
    not(test)
))]
compile_error!("feature `file_store` on wasm32 requires -Ctarget-feature=+atomics");
#[cfg(not(any(feature = "calendar", feature = "coc7th")))]
compile_error!("either feature `calendar` or `coc7th` is required");
#[cfg(any(test, not(target_arch = "wasm32")))]
extern crate std;

use alloc::{string::String, vec::Vec};
#[cfg(test)]
use core::future::Future;
#[cfg(all(target_arch = "wasm32", not(test)))]
use core::panic::PanicInfo;

#[cfg(feature = "calendar")]
use crate::calendar::data::DataError;
#[cfg(feature = "credential")]
use crate::credential::CredentialError;
#[cfg(feature = "vault")]
use crate::vault::VaultError;
use crate::{
    arena::{ArenaError, PanicError},
    event::EventError,
    file_store::FileStoreError,
};

pub mod app;
pub mod arena;
#[cfg(feature = "calendar")]
pub mod calendar;
#[cfg(feature = "coc7th")]
pub mod coc7th;
#[cfg(feature = "credential")]
pub mod credential;
pub mod data_struct;
pub mod event;
pub mod field;
pub mod file_store;
pub mod host;
pub mod list;
pub mod object;
pub mod timestamp;
#[cfg(feature = "vault")]
pub mod vault;

// === Error ===

pub trait WireError {
    fn identifiers(&self, path: &mut Vec<u16>);
    fn detail(&self) -> String;
}

macro_rules! wire_error {
    ($name:ident { $($(#[$meta:meta])* $variant:ident($inner:ty) = $identifier:expr),+ $(,)? }) => {
        #[derive(Debug)]
        pub enum $name {
            $($(#[$meta])* $variant($inner)),+
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::write!(f, "{:?}", self)
            }
        }

        impl $crate::WireError for $name {
            fn identifiers(&self, path: &mut ::alloc::vec::Vec<u16>) {
                match self {
                    $($(#[$meta])* $name::$variant(error) => {
                        path.push($identifier);
                        $crate::WireError::identifiers(error, path);
                    })+
                }
            }

            fn detail(&self) -> ::alloc::string::String {
                match self {
                    $($(#[$meta])* $name::$variant(error) => $crate::WireError::detail(error)),+
                }
            }
        }
    };
}

wire_error! {
    Error {
        Arena(ArenaError) = 1,
        Event(EventError) = 2,
        Panic(PanicError) = 3,
        FileStore(FileStoreError) = 4,
        #[cfg(feature = "calendar")]
        Data(DataError) = 5,
        #[cfg(feature = "credential")]
        Credential(CredentialError) = 8,
        #[cfg(feature = "vault")]
        Vault(VaultError) = 13,
    }
}

pub trait Input<'a>: Sized {
    fn decode(input: &mut &'a [u8]) -> Option<Self>;
}

pub trait Output {
    fn encode(&self, output: &mut Vec<u8>);
}

fn take<'a>(input: &mut &'a [u8], count: usize) -> Option<&'a [u8]> {
    let (head, tail) = input.split_at_checked(count)?;
    *input = tail;
    Some(head)
}

macro_rules! scalar {
    ($($type:ty),*) => {$(
        impl Input<'_> for $type {
            fn decode(input: &mut &[u8]) -> Option<Self> {
                Some(<$type>::from_le_bytes(take(input, size_of::<$type>())?.try_into().ok()?))
            }
        }

        impl Output for $type {
            fn encode(&self, output: &mut Vec<u8>) {
                output.extend_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}

scalar!(u8, u16, u32, u64, u128, i32, f32, f64);

macro_rules! wire_enum {
    ($($repr:ty: $($type:ty),+);+ $(;)?) => {$($(
        impl Output for $type {
            fn encode(&self, output: &mut Vec<u8>) {
                (*self as $repr).encode(output);
            }
        }
    )+)+};
}
pub(crate) use wire_enum;

impl<'a, T: Input<'a>> Input<'a> for Vec<T> {
    fn decode(input: &mut &'a [u8]) -> Option<Self> {
        let count = u32::decode(input)? as usize;
        (0..count).map(|_| T::decode(input)).collect()
    }
}

impl Input<'_> for String {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        String::from_utf8(Vec::<u8>::decode(input)?).ok()
    }
}

impl<T: Output> Output for [T] {
    fn encode(&self, output: &mut Vec<u8>) {
        (self.len() as u32).encode(output);
        for item in self {
            item.encode(output);
        }
    }
}

impl Output for str {
    fn encode(&self, output: &mut Vec<u8>) {
        self.as_bytes().encode(output);
    }
}

pub trait Decimal: Sized {
    fn from_f64(value: f64) -> Self;
    fn to_f64(&self) -> f64;
}

const FRACTION_UNIT: u64 = 1000;
const FRACTION_WIDTH: usize = 3;

impl<T: Decimal> Input<'_> for T {
    fn decode(input: &mut &[u8]) -> Option<Self> {
        let (whole, rest) = input.split_at_checked(count_leading_digits(input))?;
        let (fraction, rest) = match rest.split_first() {
            Some((b'.', tail)) => tail.split_at_checked(count_leading_digits(tail))?,
            _ => (&[][..], rest),
        };
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        *input = rest;
        let mantissa = whole
            .iter()
            .chain(fraction)
            .fold(0.0, |sum, &byte| sum * 10.0 + f64::from(byte - b'0'));
        let scale = fraction.iter().fold(1.0, |scale, _| scale * 10.0);
        Some(T::from_f64(mantissa / scale))
    }
}

impl<T: Decimal> Output for T {
    fn encode(&self, output: &mut Vec<u8>) {
        let value = self.to_f64();
        let scaled = if value.is_finite() {
            libm::round(libm::fabs(value) * FRACTION_UNIT as f64) as u64
        } else {
            0
        };
        if scaled != 0 && value.is_sign_negative() {
            output.push(b'-');
        }
        push_decimal(output, scaled / FRACTION_UNIT, 1);
        let mut fraction = scaled % FRACTION_UNIT;
        if fraction == 0 {
            return;
        }
        let mut width = FRACTION_WIDTH;
        while fraction % 10 == 0 {
            fraction /= 10;
            width -= 1;
        }
        output.push(b'.');
        push_decimal(output, fraction, width);
    }
}

fn count_leading_digits(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|byte| byte.is_ascii_digit()).count()
}

fn push_decimal(output: &mut Vec<u8>, mut value: u64, width: usize) {
    let mut digits = [b'0'; 20];
    let mut start = digits.len();
    while value > 0 || digits.len() - start < width {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    output.extend_from_slice(&digits[start..]);
}

pub fn parse<'a, T: Input<'a>>(source: &'a [u8]) -> Option<T> {
    let mut rest = source;
    let value = T::decode(&mut rest)?;
    rest.is_empty().then_some(value)
}

pub fn stringify<T: Output + ?Sized>(value: &T) -> String {
    let mut output = Vec::new();
    value.encode(&mut output);
    String::from_utf8(output).unwrap_or_default()
}

// === Global allocator ===

#[cfg(target_arch = "wasm32")]
use talc::{sync::TalcLock, wasm::*};

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: TalcLock<spinning_top::RawSpinlock, WasmGrowAndClaim, WasmBinning> =
    TalcLock::new(WasmGrowAndClaim);

// === Language ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En(En),
    Ja,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum En {
    Gb,
    Us,
}

impl Language {
    pub fn list() -> &'static [Self] {
        &[Self::En(En::Gb), Self::Ja]
    }

    pub fn number(self) -> u32 {
        match self {
            Self::En(_) => 1,
            Self::Ja => 2,
        }
    }

    pub fn display(self) -> (&'static str, &'static str) {
        match self {
            Self::En(En::Gb) => ("en-GB", "English"),
            Self::En(En::Us) => ("en-US", "English"),
            Self::Ja => ("ja", "日本語"),
        }
    }
}

// === Panic handler ===

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use alloc::format;
    use core::arch::wasm32::unreachable;

    use crate::{
        arena::{PanicError, emit, report_error},
        host::OPERATION_RELOAD,
    };

    let location = match info.location() {
        Some(location) => format!("{}:{}", location.file(), location.line()),
        None => String::from("unknown"),
    };

    report_error(Error::Panic(PanicError { location, message: format!("{}", info.message()) }));
    let _ = emit(&[OPERATION_RELOAD]);

    unreachable()
}

#[cfg(test)]
mod decimal_tests {
    use alloc::{format, string::String};

    use super::*;
    use crate::Rng;

    struct Sample(f64);

    impl Decimal for Sample {
        fn from_f64(value: f64) -> Self {
            Self(value)
        }

        fn to_f64(&self) -> f64 {
            self.0
        }
    }

    fn number(source: &str) -> Option<f64> {
        parse::<Sample>(source.as_bytes()).map(|Sample(value)| value)
    }

    #[test]
    fn random_decimal_text_agrees_with_std_and_survives_a_round_trip() {
        for seed in 0..2000 {
            let mut rng = Rng::new(seed);
            let digits = |rng: &mut Rng, count: usize| -> String {
                (0..count).map(|_| char::from(b'0' + rng.below(10) as u8)).collect()
            };
            let whole_digits = rng.below(5);
            let whole = digits(&mut rng, whole_digits);
            let dot = rng.chance(70);
            let fraction_digits = rng.below(7);
            let fraction = if dot { digits(&mut rng, fraction_digits) } else { String::new() };
            let source = format!("{whole}{}{fraction}", if dot { "." } else { "" });
            if whole.is_empty() && fraction.is_empty() {
                assert_eq!(number(&source), None, "{source:?}");
                continue;
            }
            let expected = format!(
                "{}.{}",
                if whole.is_empty() { "0" } else { &whole },
                if fraction.is_empty() { "0" } else { &fraction }
            )
            .parse::<f64>()
            .unwrap();
            assert_eq!(number(&source), Some(expected), "{source:?}");

            let signed =
                if rng.chance(50) { -expected } else { expected } + rng.below(1000) as f64 / 7000.0;
            let written = stringify(&Sample(signed));
            let parsed = written.trim_start_matches('-').parse::<f64>().unwrap();
            assert!((parsed - signed.abs()).abs() <= 0.0005 + 1e-9, "{signed} -> {written}");
            assert_eq!(
                written.starts_with('-'),
                signed < 0.0 && parsed != 0.0,
                "{signed} -> {written}"
            );
            assert_eq!(number(written.trim_start_matches('-')), Some(parsed), "{written}");
            assert_eq!(stringify(&Sample(if signed < 0.0 { -parsed } else { parsed })), written);
        }
    }

    #[test]
    fn decode_rejects_non_decimal_text() {
        for source in ["", ".", "-1", "+1", "1e3", "1.2.3", "abc", " 1", "1 ", "NaN", "inf"] {
            assert_eq!(number(source), None, "{source:?}");
        }
    }

    #[test]
    fn decode_approximates_beyond_f64_precision() {
        assert_eq!(number("9007199254740993"), Some(9007199254740992.0));
        let tiny = number("0.000000000000000000000001").unwrap();
        assert!((tiny - 1e-24).abs() < 1e-36);
    }

    #[test]
    fn decode_advances_the_cursor_only_on_success() {
        let mut rest: &[u8] = b"1.75rem";
        assert_eq!(Sample::decode(&mut rest).map(|Sample(value)| value), Some(1.75));
        assert_eq!(rest, b"rem");
        let mut rest: &[u8] = b"1.2.3";
        assert_eq!(Sample::decode(&mut rest).map(|Sample(value)| value), Some(1.2));
        assert_eq!(rest, b".3");
        let mut rest: &[u8] = b"rem";
        assert!(Sample::decode(&mut rest).is_none());
        assert_eq!(rest, b"rem");
    }

    struct Head<'a>(&'a [u8]);

    impl<'a> Input<'a> for Head<'a> {
        fn decode(input: &mut &'a [u8]) -> Option<Self> {
            let count = u8::decode(input)? as usize;
            Some(Self(take(input, count)?))
        }
    }

    #[test]
    fn a_borrowing_type_outlives_the_cursor() {
        let frame: &[u8] = b"\x03abcrest";
        let head = {
            let mut rest = frame;
            let head = Head::decode(&mut rest).unwrap();
            assert_eq!(rest, b"rest");
            head
        };
        assert_eq!(head.0, b"abc");
        assert_eq!(parse::<Head>(b"\x02ab").map(|head| head.0), Some(&b"ab"[..]));
        assert_eq!(parse::<Head>(b"\x02abc").map(|head| head.0), None);
        assert!(Head::decode(&mut &b"\x05ab"[..]).is_none());
    }

    #[test]
    fn encode_rounds_to_three_fraction_digits() {
        assert_eq!(stringify(&Sample(1.23456)), "1.235");
        assert_eq!(stringify(&Sample(0.0004)), "0");
        assert_eq!(stringify(&Sample(f64::NAN)), "0");
        assert_eq!(stringify(&Sample(f64::INFINITY)), "0");
    }
}

#[cfg(test)]
mod error_tests {
    use alloc::{string::String, vec, vec::Vec};

    use super::*;

    #[test]
    fn every_error_has_a_stable_wire_path_and_detail() {
        let message = || String::from("m");
        let mut cases: Vec<(Error, Vec<u16>, &str)> = vec![
            (Error::Arena(ArenaError::CommandOverflow), vec![1, 1], ""),
            (Error::Event(EventError::Decode), vec![2, 1], ""),
            (
                Error::Panic(PanicError { location: String::from("a.rs:1"), message: message() }),
                vec![3, 1],
                "a.rs:1: m",
            ),
        ];
        cases.extend([
            (Error::FileStore(FileStoreError::InvalidState(message())), vec![4, 1], "m"),
            (Error::FileStore(FileStoreError::QuotaExceeded(message())), vec![4, 2], "m"),
            (Error::FileStore(FileStoreError::UnsupportedOp(message())), vec![4, 3], "m"),
            (Error::FileStore(FileStoreError::InvalidName(message())), vec![4, 4], "m"),
            (Error::FileStore(FileStoreError::Unknown(message())), vec![4, 5], "m"),
            (Error::FileStore(FileStoreError::NotFound(message())), vec![4, 6], "m"),
            (Error::FileStore(FileStoreError::Locked(message())), vec![4, 7], "m"),
        ]);
        #[cfg(feature = "calendar")]
        cases.extend([
            (Error::Data(DataError::Status(404)), vec![5, 1], "404"),
            (Error::Data(DataError::Parse(message())), vec![5, 2], "m"),
            (Error::Data(DataError::Format(message())), vec![5, 3], "m"),
        ]);
        #[cfg(feature = "credential")]
        cases.extend([
            (Error::Credential(CredentialError::Email), vec![8, 1], "Email"),
            (Error::Credential(CredentialError::Password), vec![8, 2], "Password"),
            (Error::Credential(CredentialError::Stretch), vec![8, 3], "Stretch"),
        ]);
        #[cfg(feature = "vault")]
        cases.extend([
            (Error::Vault(VaultError::Length(1)), vec![13, 2], "Length(1)"),
            (Error::Vault(VaultError::Open), vec![13, 3], "Open"),
        ]);
        for (error, path, detail) in cases {
            let mut actual = Vec::new();
            error.identifiers(&mut actual);
            assert_eq!(actual, path, "{error}");
            assert_eq!(error.detail(), detail, "{error}");
        }
    }
}

// === For test ===

#[cfg(test)]
fn block_on<F: Future>(future: F) -> F::Output {
    use core::{
        pin::pin,
        task::{Context, Poll, Waker},
    };

    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}

#[cfg(test)]
struct Rng(u64);

#[cfg(test)]
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() >> 11) as usize % bound
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next_u64() as u8).collect()
    }

    fn string(&mut self) -> String {
        let letters = ['a', 'Z', 'é', '日', '😀', ' ', '\0'];
        let length = self.below(10);
        (0..length).map(|_| letters[self.below(letters.len())]).collect()
    }

    fn id(&mut self) -> host::dom::Id {
        use host::dom;

        let depth = self.below(5);
        let segments: Vec<(dom::Tag, Option<u32>)> = (0..depth)
            .map(|_| {
                let tag = dom::Tag::from_u8(self.below(24) as u8);
                let number =
                    self.chance(50).then(|| (self.next_u64() % u64::from(u32::MAX)) as u32);
                (tag, number)
            })
            .collect();
        dom::Id::new(&segments)
    }
}
