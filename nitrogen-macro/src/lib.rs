use proc_macro::TokenStream;

mod rpc;

#[proc_macro_attribute]
pub fn rpc_service(attr: TokenStream, input: TokenStream) -> TokenStream {
    rpc::rpc_service(attr, input)
}
