use std::{collections::HashMap, net::SocketAddr, pin::Pin, sync::Arc};

use futures::{Future, FutureExt, StreamExt};
use nitrogen_quic::{create_client, BidirectionalStream, Client, Connect, Connection};
use nitrogen_utils::FramedTokioIO;
use parking_lot::Mutex;
use tokio_util::codec::LengthDelimitedCodec;

use crate::{Negotiate, Session};

#[derive(Clone)]
pub struct Nitrogen {
    client: Client,
    sessions: Arc<Mutex<HashMap<SocketAddr, Session>>>,
    services: Arc<Mutex<HashMap<String, ServiceHandler>>>,
    server_addr: Option<SocketAddr>,
}

pub type ServiceHandler = Arc<dyn Fn(FramedTokioIO<BidirectionalStream>, Session, Nitrogen) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

impl Nitrogen {
    pub async fn new() -> Result<Self, anyhow::Error> {
        Ok(Self {
            client: create_client("0.0.0.0:0".parse::<SocketAddr>()?).await?,
            sessions: Default::default(),
            services: Default::default(),
            server_addr: None,
        })
    }
}

impl Nitrogen {
    pub fn local_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.client.local_addr()?)
    }

    pub fn server_addr(&self) -> anyhow::Result<SocketAddr> {
        self.server_addr.ok_or(anyhow::anyhow!("no server addr"))
    }

    pub async fn connect(&self, addr: SocketAddr) -> anyhow::Result<Session> {
        if let Some(session) = self.sessions.lock().get(&addr).cloned() {
            return Ok(session);
        }

        let mut connection = self.client.connect(Connect::new(addr).with_server_name("localhost")).await?;
        connection.keep_alive(true)?;
        self.spawn_accept(connection)?;
        let session = self.sessions.lock().get(&addr).cloned().ok_or(anyhow::anyhow!("no session"))?;
        Ok(session)
    }

    pub async fn serve(mut self, addr: SocketAddr) -> anyhow::Result<Self> {
        let this = self.clone();

        let mut server = nitrogen_quic::create_server(addr).await?;

        self.server_addr = Some(server.local_addr()?);

        tokio::spawn(async move {
            while let Some(connection) = server.accept().await {
                this.spawn_accept(connection)?;
            }
            anyhow::Result::<()>::Ok(())
        });

        Ok(self)
    }
}

impl Nitrogen {
    fn spawn_accept(&self, mut connection: Connection) -> anyhow::Result<()> {
        let addr = connection.remote_addr()?;
        let sessions = self.sessions.clone();
        sessions.lock().insert(addr, Session::new(connection.handle()));

        let this = self.clone();

        tokio::spawn(async move {
            while let Ok(Some(bi_stream)) = connection.accept_bidirectional_stream().await {
                let this = this.clone();

                tokio::spawn(async move {
                    let mut framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(bi_stream);

                    let bytes = framed_io.next().await.ok_or(anyhow::anyhow!("no message"))??;
                    let negotiate = rmp_serde::from_slice::<Negotiate>(&bytes)?;

                    let service_handler = this.services.lock().get(&negotiate.name).ok_or(anyhow::anyhow!("no service"))?.clone();

                    let session = this.sessions.lock().get(&addr).ok_or(anyhow::anyhow!("no session"))?.clone();
                    service_handler(framed_io, session, this).await;

                    anyhow::Result::<()>::Ok(())
                });
            }

            sessions.lock().remove(&addr);
        });

        Ok(())
    }
}

impl Nitrogen {
    pub fn add_service<H, Fut>(self, name: &str, handler: H) -> Self
    where
        H: Fn(FramedTokioIO<BidirectionalStream>, Session, Nitrogen) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.services.lock().insert(
            name.into(),
            Arc::new(move |bi_stream, session, nitrogen| handler(bi_stream, session, nitrogen).boxed()),
        );
        self
    }

    pub fn services(&self) -> Vec<String> {
        self.services.lock().keys().cloned().collect()
    }
}
