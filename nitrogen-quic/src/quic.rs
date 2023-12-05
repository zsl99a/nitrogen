use std::net::SocketAddr;

use s2n_quic::{provider::event::default::Subscriber, Client, Server};

use crate::MtlsProvider;

pub static CA_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.crt");
pub static MY_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.crt");
pub static MY_KEY_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

pub async fn create_client(addr: SocketAddr) -> anyhow::Result<Client> {
    let mtls = MtlsProvider::new(CA_CERT_PEM, MY_CERT_PEM, MY_KEY_PEM).await?;
    let client = Client::builder().with_event(Subscriber::default())?.with_tls(mtls)?.with_io(addr)?.start()?;
    Ok(client)
}

pub async fn create_server(addr: SocketAddr) -> anyhow::Result<Server> {
    let mtls = MtlsProvider::new(CA_CERT_PEM, MY_CERT_PEM, MY_KEY_PEM).await?;
    let server = Server::builder().with_event(Subscriber::default())?.with_tls(mtls)?.with_io(addr)?.start()?;
    Ok(server)
}
