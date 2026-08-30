use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

fn unit_variant_idents(input: &DeriveInput) -> Vec<&syn::Ident> {
    let Data::Enum(data) = &input.data else {
        panic!("this derive only supports enums");
    };
    data.variants
        .iter()
        .map(|v| {
            if !matches!(v.fields, Fields::Unit) {
                panic!("this derive only supports fieldless (unit) variants");
            }
            &v.ident
        })
        .collect()
}

/// Add pub fn list() -> &'static [Self]
#[proc_macro_derive(List)]
pub fn derive_list(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = &input.ident;
    let variants = unit_variant_idents(&input);

    quote! {
        impl #ty {
            pub fn list() -> &'static [#ty] {
                &[#(Self::#variants),*]
            }
        }
    }
    .into()
}

/// Add pub fn id(self) -> u8 (order in script)
#[proc_macro_derive(Id)]
pub fn derive_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = &input.ident;
    let variants = unit_variant_idents(&input);
    let ids = 1u8..=(variants.len() as u8);

    quote! {
        impl #ty {
            pub fn id(self) -> u8 {
                match self {
                    #(Self::#variants => #ids),*
                }
            }
        }
    }
    .into()
}

/// Add pub fn roll() -> Self` (random)
/// dependency: list()
///
/// 生成するコードは `rand::rng()` を使わない。`rand::rng()` は
/// `#[cfg(feature = "thread_rng")]` であり `thread_rng = ["std", ..]` の
/// ため、`no_std` では解決できないからである。この derive を使う側すべてに
/// 波及するので、生成側で断つ必要がある。
///
/// 代わりに `sys_rng` feature の `SysRng` で seed を取り `SmallRng` を回す。
/// `SysRng` は fallible (`TryRng`) であり `RngExt` が付かないため、
/// 直接 `random_range` を呼ぶことはできない。
#[proc_macro_derive(Roll)]
pub fn derive_roll(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = &input.ident;
    let _ = unit_variant_idents(&input);

    quote! {
        impl #ty {
            pub fn roll() -> Self {
                use rand::{RngExt as _, SeedableRng as _, TryRng as _};

                let mut seed = [0u8; 32];
                let mut sys  = rand::rngs::SysRng::default();
                sys.try_fill_bytes(&mut seed).unwrap();
                let mut rng = rand::rngs::SmallRng::from_seed(seed);

                let list = Self::list();
                list[rng.random_range(0..list.len())]
            }
        }
    }
    .into()
}
