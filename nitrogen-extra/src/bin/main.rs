use std::time::Duration;

use nitrogen::{BiConnect, BiConnnectionAcceptor, BiConnnectionOpener, BiConnnectionSplit, BiListener, Negotiator, RpcServiceClient};
use nitrogen_quic::{QuicConnect, QuicListener};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tokio::spawn(server());
    tokio::time::sleep(Duration::from_millis(1)).await;
    tokio::spawn(client());

    tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
    Ok(())
}

async fn server() -> anyhow::Result<()> {
    let client = QuicConnect::bind("0.0.0.0:0".parse()?).await?;
    let mut server = QuicListener::bind("0.0.0.0:31234".parse()?).await?;

    while let Ok(connection) = server.accept().await {
        let client = client.clone();
        let (_opener, mut acceptor) = connection.split();

        tokio::spawn(async move {
            while let Ok(mut bi_stream) = acceptor.accept().await {
                let _client = client.clone();

                tokio::spawn(async move {
                    let service_name = Negotiator::<String>::new().recv(&mut bi_stream).await?;

                    match service_name.as_str() {
                        MyServiceImpl::NAME => MyServiceImpl.serve(bi_stream).await,
                        _ => {
                            anyhow::bail!("unknown service: {}", service_name)
                        }
                    }

                    Ok::<(), anyhow::Error>(())
                });
            }
        });
    }

    Ok(())
}

async fn client() -> anyhow::Result<()> {
    let mut client = QuicConnect::bind("0.0.0.0:0".parse()?).await?;

    let mut connect = client.connect("127.0.0.1:31234".parse()?).await?;
    let mut stream = connect.open().await?;

    Negotiator::<String>::new().send(&mut stream, MyServiceClient::NAME.into()).await?;

    let svc_client = MyServiceClient::new(stream);
    let msg = svc_client.ping(vec![1, 2, 3]).await?;
    println!("ping: {}", msg);

    Ok(())
}

#[nitrogen::rpc_service]
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
        "|single|".to_string()
    }
}
