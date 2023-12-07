use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum, ItemTrait};

/// #[rpc_service]
/// pub trait MyService {
///     async fn fn_name(&self, arg1: Arg1, arg2: Arg2, arg3: Arg3) -> Return;
///     async fn fn_name2(&self);
/// }
pub fn rpc_service(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemTrait);

    set_supertraits(&mut input);

    let request_enum = make_request_enum(&input);
    let response_enum = make_response_enum(&input);

    let ext_trait = make_ext_trait(&input);
    let ext_impl = make_ext_impl(&input);

    let client_struct = make_client_struct(&input);
    let client_impl_new = make_client_impl_new(&input);
    let client_impl_trait = make_client_impl_trait(&input);
    let client_impl_fn = make_client_impl_fn(&input);

    let output = quote!(
        #[async_trait::async_trait]
        #input

        #request_enum
        #response_enum

        #ext_trait
        #ext_impl

        #client_struct
        #client_impl_new
        #client_impl_trait
        #client_impl_fn
    );

    TokenStream::from(output)
}

// --- 设置基础特征 ---

/// #[async_trait::async_trait]
/// pub trait MyService: Clone + Send + Sync + 'static {
///     const NAME: &'static str = "MyService";
///     async fn fn_name(&self, arg1: Arg1, arg2: Arg2, arg3: Arg3) -> Return;
///     async fn fn_name2(&self);
/// }
fn set_supertraits(input: &mut ItemTrait) {
    input.supertraits.push(syn::parse_quote!(Clone));
    input.supertraits.push(syn::parse_quote!(Send));
    input.supertraits.push(syn::parse_quote!(Sync));
    input.supertraits.push(syn::parse_quote!('static));

    let name = input.ident.to_string();
    input.items.push(syn::parse_quote!(const NAME: &'static str = #name;));
}

// --- 生成 request 和 response 枚举 ---

/// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// pub enum MyServiceRequest {
///     FnName(Arg1, Arg2, Arg3),
///     FnName2,
/// }
fn make_request_enum(input: &ItemTrait) -> ItemEnum {
    let request_enum_ident = make_request_enum_ident(input);

    let request_enum_items = input.items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(item_fn) = item {
            let item_ty_ident = to_camel_case(&format!("{}", item_fn.sig.ident));
            let request_item_ident = syn::Ident::new(&item_ty_ident, item_fn.sig.ident.span());

            let fn_inputs = item_fn
                .sig
                .inputs
                .iter()
                .filter_map(|fn_input| match fn_input {
                    syn::FnArg::Receiver(_receiver) => None,
                    syn::FnArg::Typed(pat_type) => Some(pat_type.ty.clone()),
                })
                .collect::<Vec<_>>();

            let output = if fn_inputs.is_empty() {
                quote!( #request_item_ident )
            } else {
                quote!( #request_item_ident(#(#fn_inputs),*) )
            };
            Some(output)
        } else {
            None
        }
    });

    let request_enum = quote!(
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub enum #request_enum_ident { #(#request_enum_items),* }
    );

    syn::parse(TokenStream::from(request_enum)).unwrap()
}

/// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// pub enum MyServiceResponse {
///     FnName(Result<Return>),
///     FnName2(Result<()>),
/// }
fn make_response_enum(input: &ItemTrait) -> ItemEnum {
    let response_enum_ident = make_response_enum_ident(input);

    let response_enum_items = input.items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(item_fn) = item {
            let item_ty_ident = to_camel_case(&format!("{}", item_fn.sig.ident));
            let response_item_ident = syn::Ident::new(&item_ty_ident, item_fn.sig.ident.span());

            let fn_output = &item_fn.sig.output;

            let output = if let syn::ReturnType::Type(_ra, ty) = fn_output {
                quote!( #response_item_ident(nitrogen::Result<#ty>) )
            } else {
                quote!( #response_item_ident(nitrogen::Result<()>) )
            };
            Some(output)
        } else {
            None
        }
    });

    let response_enum = quote!(
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub enum #response_enum_ident { #(#response_enum_items),* }
    );

    syn::parse(TokenStream::from(response_enum)).unwrap()
}

// --- 生成服务扩展 ---

/// #[async_trait::async_trait]
/// pub trait MyServiceExt<Req, Resp>: MyService
/// where
///     Req: serde::de::DeserializeOwned + Send + 'static,
///     Resp: serde::Serialize + Send + 'static,
/// {
///     async fn route(&self, req: Req) -> Resp;
///
///     async fn serve<S>(self, framed_io: nitrogen::FramedTokioIO<S>)
///     where
///         S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
///     {
///         use futures::{SinkExt, StreamExt};
///
///         let (sender, mut receiver) = nitrogen::framed_message_pack::<nitrogen::Message<Req>, nitrogen::Message<Resp>, S>(framed_io).split();
///         let sender = nitrogen::channel_sender_with_sink(sender);
///         while let Some(Ok(nitrogen::Message { id, payload })) = receiver.next().await {
///             let this = self.clone();
///             let mut sender = sender.clone();
///             tokio::spawn(async move {
///                 let payload = this.route(payload).await;
///                 let _ = sender.send(nitrogen::Message { id, payload }).await;
///             });
///         }
///     }
/// }
fn make_ext_trait(input: &ItemTrait) -> proc_macro2::TokenStream {
    let trait_ident = input.ident.clone();
    let ext_trait_ident = make_ext_trait_ident(input);

    let output = quote!(
        #[async_trait::async_trait]
        pub trait #ext_trait_ident<Req, Resp>: #trait_ident
        where
            Req: serde::de::DeserializeOwned + Send + 'static,
            Resp: serde::Serialize + Send + 'static,
        {
            async fn route(&self, req: Req) -> Resp;

            async fn serve<S>(self, stream: S)
            where
                S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
            {
                use futures::{SinkExt, StreamExt};

                let framed_io = tokio_util::codec::LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(stream);
                let (sender, mut receiver) = nitrogen::framed_message_pack::<nitrogen::Message<Req>, nitrogen::Message<Resp>, S>(framed_io).split();
                let sender = nitrogen::channel_sender_with_sink(sender);
                while let Some(Ok(nitrogen::Message { id, payload })) = receiver.next().await {
                    let this = self.clone();
                    let mut sender = sender.clone();
                    tokio::spawn(async move {
                        let payload = this.route(payload).await;
                        let _ = sender.send(nitrogen::Message { id, payload }).await;
                    });
                }
            }
        }
    );

    output
}

/// #[async_trait::async_trait]
/// impl<T> MyServiceExt<MyServiceRequest, MyServiceResponse> for T
/// where
///     T: MyService,
/// {
///     async fn route(&self, req: MyServiceRequest) -> MyServiceResponse {
///         match req {
///             MyServiceRequest::FnName(arg1, arg2, arg3) => MyServiceResponse::FnName(Ok(self.fn_name(arg1, arg2, arg3).await)),
///             MyServiceRequest::FnName2 => MyServiceResponse::FnName2(Ok(self.fn_name2().await)),
///         }
///     }
/// }
fn make_ext_impl(input: &ItemTrait) -> proc_macro2::TokenStream {
    let trait_ident = input.ident.clone();
    let ext_trait_ident = make_ext_trait_ident(input);
    let request_enum_ident = make_request_enum_ident(input);
    let response_enum_ident = make_response_enum_ident(input);

    let ext_enum_match = input.items.iter().filter_map(|item| {
        // MyServiceRequest::FnName(arg1, arg2, arg3) => MyServiceResponse::FnName(Ok(self.fn_name(arg1, arg2, arg3).await)),
        // Or:
        // MyServiceRequest::FnName2 => MyServiceResponse::FnName2(Ok(self.fn_name2().await)),

        if let syn::TraitItem::Fn(item_fn) = item {
            let ident_name = to_camel_case(&format!("{}", item_fn.sig.ident));
            let enum_item_ident = syn::Ident::new(&ident_name, item_fn.sig.ident.span());
            let fn_item_ident = syn::Ident::new(&format!("{}", item_fn.sig.ident), item_fn.sig.ident.span());

            let fn_inputs = item_fn
                .sig
                .inputs
                .iter()
                .filter_map(|fn_input| match fn_input {
                    syn::FnArg::Receiver(_receiver) => None,
                    syn::FnArg::Typed(pat_type) => Some(pat_type.pat.clone()),
                })
                .collect::<Vec<_>>();

            let output = if fn_inputs.is_empty() {
                quote!( #request_enum_ident::#enum_item_ident => #response_enum_ident::#enum_item_ident(Ok(self.#fn_item_ident().await)) )
            } else {
                quote!( #request_enum_ident::#enum_item_ident(#(#fn_inputs),*) => #response_enum_ident::#enum_item_ident(Ok(self.#fn_item_ident(#(#fn_inputs),*).await)) )
            };

            Some(output)
        } else {
            None
        }
    });

    let output = quote!(
        #[async_trait::async_trait]
        impl<T> #ext_trait_ident<#request_enum_ident, #response_enum_ident> for T
        where
            T: #trait_ident,
        {
            async fn route(&self, req: #request_enum_ident) -> #response_enum_ident {
                match req { #(#ext_enum_match),* }
            }
        }
    );

    output
}

// --- 生成客户端实现 ---

/// #[derive(Clone)]
/// pub struct MyServiceClient {
///     tx: futures::channel::mpsc::Sender<(MyServiceRequest, futures::channel::oneshot::Sender<MyServiceResponse>)>,
/// }
fn make_client_struct(input: &ItemTrait) -> proc_macro2::TokenStream {
    let client_ident = make_client_ident(input);
    let request_enum_ident = make_request_enum_ident(input);
    let response_enum_ident = make_response_enum_ident(input);

    let output = quote!(
        #[derive(Clone)]
        pub struct #client_ident {
            tx: futures::channel::mpsc::Sender<(#request_enum_ident, futures::channel::oneshot::Sender<#response_enum_ident>)>,
        }
    );

    output
}

/// impl MyServiceClient
/// where
///     S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
/// {
///     pub fn new(session: Session) -> Self {
///         use futures::channel::{mpsc, oneshot};
///         use nitrogen::RpcServiceClient;
///         let (tx, rx) = mpsc::channel::<(MyServiceRequest, oneshot::Sender<MyServiceResponse>)>(128);
///         Self { tx }.spawn(session, rx)
///     }
/// }
fn make_client_impl_new(input: &ItemTrait) -> proc_macro2::TokenStream {
    let client_ident = make_client_ident(input);
    let request_enum_ident = make_request_enum_ident(input);
    let response_enum_ident = make_response_enum_ident(input);

    let output = quote!(
        impl #client_ident {
            pub fn new<S>(stream: S) -> Self
            where
                S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
             {
                use futures::channel::{mpsc, oneshot};
                use nitrogen::RpcServiceClient;
                let (tx, rx) = mpsc::channel::<(#request_enum_ident, oneshot::Sender<#response_enum_ident>)>(128);
                Self { tx }.spawn(rx, stream)
            }
        }
    );

    output
}

/// impl nitrogen::RpcServiceClient<MyServiceRequest, MyServiceResponse> for MyServiceClient {
///     const NAME: &'static str = "MyService";
///
///     fn tx(&self) -> futures::channel::mpsc::Sender<(MyServiceRequest, futures::channel::oneshot::Sender<MyServiceResponse>)> {
///         self.tx.clone()
///     }
/// }
fn make_client_impl_trait(input: &ItemTrait) -> proc_macro2::TokenStream {
    let trait_ident = input.ident.clone();
    let client_ident = make_client_ident(input);
    let request_enum_ident = make_request_enum_ident(input);
    let response_enum_ident = make_response_enum_ident(input);

    let output = quote!(
        impl nitrogen::RpcServiceClient<#request_enum_ident, #response_enum_ident> for #client_ident {
            const NAME: &'static str = stringify!(#trait_ident);

            fn tx(&self) -> futures::channel::mpsc::Sender<(#request_enum_ident, futures::channel::oneshot::Sender<#response_enum_ident>)> {
                self.tx.clone()
            }
        }
    );

    output
}

/// impl MyServiceClient {
///     pub async fn fn_name(&self, arg1: Arg1, arg2: Arg2, arg3: Arg3) -> nitrogen::Result<Return> {
///         use nitrogen::RpcServiceClient;
///         let resp = self.request(MyServiceRequest::FnName(arg1, arg2, arg3)).await?;
///         match resp {
///             MyServiceResponse::FnName(res) => res,
///             _ => Err(nitrogen::Error(format!("{}::{} error: {:?}", "MyServiceRequest", "fn_name", resp))),
///         }
///     }
///
///     pub async fn fn_name2(&self) -> nitrogen::Result<()> {
///         use nitrogen::RpcServiceClient;
///         let resp = self.request(MyServiceRequest::FnName2).await?;
///         match resp {
///             MyServiceResponse::FnName2(res) => res,
///             _ => Err(nitrogen::Error(format!("{}::{} error: {:?}", "MyServiceRequest", "fn_name2", resp))),
///         }
///     }
/// }
fn make_client_impl_fn(input: &ItemTrait) -> proc_macro2::TokenStream {
    let client_ident = make_client_ident(input);
    let request_enum_ident = make_request_enum_ident(input);
    let response_enum_ident = make_response_enum_ident(input);

    let client_impl_fn = input.items.iter().filter_map(|item| {
        if let syn::TraitItem::Fn(item_fn) = item {
            // pub async fn fn_name(&self, arg1: Arg1, arg2: Arg2, arg3: Arg3) -> nitrogen::Result<Return> {
            //     use nitrogen::RpcServiceClient;
            //     let resp = self.request(MyServiceRequest::FnName(arg1, arg2, arg3)).await?;
            //     match resp {
            //         MyServiceResponse::FnName(res) => res,
            //         _ => Err(nitrogen::Error(format!("{}::{} error: {:?}", "MyServiceRequest", "fn_name", resp))),
            //     }
            // }
            let fn_name_ident = item_fn.sig.ident.clone();
            let item_ty_str = to_camel_case(&format!("{}", fn_name_ident));
            let request_item_ident = syn::Ident::new(&item_ty_str, fn_name_ident.span());
            let response_item_ident = syn::Ident::new(&item_ty_str, fn_name_ident.span());

            let fn_sig_inputs = item_fn.sig.inputs.iter().cloned().collect::<Vec<_>>();

            let fn_args_idents = item_fn
                .sig
                .inputs
                .iter()
                .filter_map(|fn_input| match fn_input {
                    syn::FnArg::Receiver(_receiver) => None,
                    syn::FnArg::Typed(pat_type) => Some(pat_type.pat.clone()),
                })
                .collect::<Vec<_>>();

            let fn_result_ty = if let syn::ReturnType::Type(_ra, ty) = &item_fn.sig.output {
                ty.clone()
            } else {
                syn::parse_quote!(())
            };

            let resp_args = if fn_args_idents.is_empty() {
                quote!( #request_enum_ident::#request_item_ident )
            } else {
                quote!( #request_enum_ident::#request_item_ident(#(#fn_args_idents),*) )
            };

            let output = quote!(
                pub async fn #fn_name_ident(#(#fn_sig_inputs),*) -> nitrogen::Result<#fn_result_ty> {
                    use nitrogen::RpcServiceClient;
                    let resp = self.request(#resp_args).await?;
                    match resp {
                        #response_enum_ident::#response_item_ident(res) => res,
                        _ => Err(nitrogen::Error(format!("{}::{} error: {:?}", stringify!(#request_enum_ident), stringify!(#fn_name_ident), resp))),
                    }
                }
            );

            Some(output)
        } else {
            None
        }
    });

    let output = quote!(
        impl #client_ident {
            #(#client_impl_fn)*
        }
    );

    output
}

// --- make_*_ident ---

fn make_request_enum_ident(input: &ItemTrait) -> syn::Ident {
    syn::Ident::new(&format!("{}Request", input.ident), input.ident.span())
}

fn make_response_enum_ident(input: &ItemTrait) -> syn::Ident {
    syn::Ident::new(&format!("{}Response", input.ident), input.ident.span())
}

fn make_ext_trait_ident(input: &ItemTrait) -> syn::Ident {
    syn::Ident::new(&format!("{}Ext", input.ident), input.ident.span())
}

fn make_client_ident(input: &ItemTrait) -> syn::Ident {
    syn::Ident::new(&format!("{}Client", input.ident), input.ident.span())
}

// --- 工具函数 ---

// 下划线变量名转驼峰变量名
fn to_camel_case(name: &str) -> String {
    let mut result = String::new();

    for (i, c) in name.chars().enumerate() {
        if i == 0 {
            result.push(c.to_ascii_uppercase());
        } else if c == '_' {
            continue;
        } else if name.chars().nth(i - 1).unwrap() == '_' {
            result.push(c.to_ascii_uppercase());
        } else {
            result.push(c);
        }
    }

    result
}
