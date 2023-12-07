use std::net::SocketAddr;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

#[async_trait]
pub trait BiListener
where
    Self: Sized,
{
    type Connection: BiConnnectionSplit;

    async fn accept(&mut self) -> anyhow::Result<Self::Connection>;
}

#[async_trait]
pub trait BiConnect {
    type Connection: BiConnnectionSplit;

    async fn connect(&mut self, addr: SocketAddr) -> anyhow::Result<Self::Connection>;
}

#[async_trait]
pub trait BiConnnectionAcceptor {
    type Stream: AsyncRead + AsyncWrite;

    async fn accept(&mut self) -> anyhow::Result<Self::Stream>;
}

#[async_trait]
pub trait BiConnnectionOpener {
    type Stream: AsyncRead + AsyncWrite + Send + Unpin + 'static;

    async fn open(&mut self) -> anyhow::Result<Self::Stream>;
}

pub trait BiConnnectionSplit {
    type Opener: BiConnnectionOpener;
    type Acceptor: BiConnnectionAcceptor;

    fn split(self) -> (Self::Opener, Self::Acceptor);
}

pub trait BiStreamSplit {
    type Write: AsyncWrite;
    type Read: AsyncRead;

    fn split(self) -> (Self::Write, Self::Read);
}
