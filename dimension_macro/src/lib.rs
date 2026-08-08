use proc_macro::TokenStream;
use quote::quote;
use syn::{LitInt, parse_macro_input};

#[proc_macro]
pub fn dimensions(input: TokenStream) -> TokenStream {
    let dimensions = parse_macro_input!(input as LitInt);
    let n = dimensions
        .base10_parse::<usize>()
        .expect("DIMENSIONS must be a usize");

    let mut ty = quote!(u16);

    for _ in 0..n {
        ty = quote!(Vec<#ty>);
    }

    quote! {
        const DIMENSIONS: usize = #n;
        type ResourcesType = #ty;
    }
    .into()
}
