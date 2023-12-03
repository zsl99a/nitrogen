use proc_macro::TokenStream;

mod pub_sub;
mod rpc;

#[proc_macro_attribute]
pub fn rpc_service(attr: TokenStream, input: TokenStream) -> TokenStream {
    rpc::rpc_service(attr, input)
}

#[proc_macro_attribute]
pub fn pub_sub_service(attr: TokenStream, input: TokenStream) -> TokenStream {
    pub_sub::pub_sub_service(attr, input)
}
