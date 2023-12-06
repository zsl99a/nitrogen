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
    type Stream: AsyncRead + AsyncWrite;

    async fn open(&mut self) -> anyhow::Result<Self::Stream>;
}

pub trait BiConnnectionSplit {
    type Acceptor: BiConnnectionAcceptor;
    type Opener: BiConnnectionOpener;

    fn split(self) -> (Self::Acceptor, Self::Opener);
}

pub trait BiStreamSplit {
    type Read: AsyncRead;
    type Write: AsyncWrite;

    fn split(self) -> (Self::Read, Self::Write);
}
