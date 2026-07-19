use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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
#[proc_macro_derive(Roll)]
pub fn derive_roll(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ty = &input.ident;
    let _ = unit_variant_idents(&input);

    quote! {
        impl #ty {
            pub fn roll() -> Self {
                use rand::RngExt as _;
                let mut rng = rand::rng();
                let list = Self::list();
                list[rng.random_range(0..list.len())]
            }
        }
    }
    .into()
}
