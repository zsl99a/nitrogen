use std::error::Error;

use futures::{SinkExt, StreamExt};
use nitrogen::Nitrogen;
use nitrogen_utils::framed_message_pack;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let nitrogen = Nitrogen::new()
        .await?
        .add_service(MyServiceImpl::NAME, |framed_io, _session| MyServiceImpl.serve(framed_message_pack(framed_io)))
        .serve("0.0.0.0:31234".parse()?)
        .await?;

    let session = nitrogen.connect("127.0.0.1:31234".parse()?).await?;
    let client = MyServiceClient::new(session);

    let ping = client.ping(b"Didi".to_vec()).await?;
    println!("ping: {}", ping);

    Ok(())
}

#[nitrogen_macro::rpc_service]
pub trait MyService {
    async fn ping(&self, time: Vec<u8>) -> String;
    async fn hello(&self, name: String) -> String;
    async fn single(&self) -> String;
}

#[derive(Clone)]
pub struct MyServiceImpl;

#[async_trait::async_trait]
impl MyService for MyServiceImpl {
    async fn ping(&self, time: Vec<u8>) -> String {
        format!("|name: {}, time: {:?}|", Self::NAME, time)
    }

    async fn hello(&self, name: String) -> String {
        format!("|hello, {}|", name)
    }

    async fn single(&self) -> String {
        format!("|single|")
    }
}
