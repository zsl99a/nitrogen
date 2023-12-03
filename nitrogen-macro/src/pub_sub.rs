use proc_macro::TokenStream;
use quote::quote;
// use syn::{parse_macro_input, ItemEnum, ItemTrait};

pub fn pub_sub_service(_attr: TokenStream, _input: TokenStream) -> TokenStream {
    let output = quote!();

    TokenStream::from(output)
}
