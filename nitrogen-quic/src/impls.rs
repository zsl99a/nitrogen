use std::{
    io::Result,
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use nitrogen_utils::{BiConnect, BiConnnectionAcceptor, BiConnnectionOpener, BiConnnectionSplit, BiListener, BiStreamSplit};
use s2n_quic::{
    client::Connect,
    connection::{Handle, StreamAcceptor},
    stream::{BidirectionalStream, ReceiveStream, SendStream},
    Client, Connection, Server,
};
use tokio::io::ReadBuf;

use crate::quic::{create_client, create_server};

// --- QuicStream ---

pin_project_lite::pin_project! {
    pub struct QuicStream {
        #[pin]
        stream: BidirectionalStream,
    }
}

impl tokio::io::AsyncRead for QuicStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<Result<()>> {
        self.project().stream.poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for QuicStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.project().stream.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().stream.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.project().stream.poll_shutdown(cx)
    }
}

impl BiStreamSplit for QuicStream {
    type Write = SendStream;
    type Read = ReceiveStream;

    fn split(self) -> (Self::Write, Self::Read) {
        let (recv, send) = self.stream.split();
        (send, recv)
    }
}

// --- QuicConnection ---

pub struct QuicConnection {
    connection: Connection,
}

#[async_trait]
impl BiConnnectionAcceptor for QuicConnection {
    type Stream = QuicStream;

    async fn accept(&mut self) -> anyhow::Result<Self::Stream> {
        let stream = self.connection.accept_bidirectional_stream().await?.ok_or(anyhow::anyhow!("no stream"))?;
        Ok(QuicStream { stream })
    }
}

#[async_trait]
impl BiConnnectionOpener for QuicConnection {
    type Stream = QuicStream;

    async fn open(&mut self) -> anyhow::Result<Self::Stream> {
        let stream = self.connection.open_bidirectional_stream().await?;
        Ok(QuicStream { stream })
    }
}

impl BiConnnectionSplit for QuicConnection {
    type Opener = QuicConnectionOpener;
    type Acceptor = QuicConnectionAcceptor;

    fn split(self) -> (Self::Opener, Self::Acceptor) {
        let (opener, acceptor) = self.connection.split();
        (QuicConnectionOpener { opener }, QuicConnectionAcceptor { acceptor })
    }
}

// --- QuicConnectionAcceptor ---

pub struct QuicConnectionAcceptor {
    acceptor: StreamAcceptor,
}

#[async_trait]
impl BiConnnectionAcceptor for QuicConnectionAcceptor {
    type Stream = QuicStream;

    async fn accept(&mut self) -> anyhow::Result<Self::Stream> {
        let stream = self.acceptor.accept_bidirectional_stream().await?.ok_or(anyhow::anyhow!("no stream"))?;
        Ok(QuicStream { stream })
    }
}

// --- QuicConnectionOpener ---

#[derive(Clone)]
pub struct QuicConnectionOpener {
    opener: Handle,
}

#[async_trait]
impl BiConnnectionOpener for QuicConnectionOpener {
    type Stream = QuicStream;

    async fn open(&mut self) -> anyhow::Result<Self::Stream> {
        let stream = self.opener.open_bidirectional_stream().await?;
        Ok(QuicStream { stream })
    }
}

// --- QuicListener ---

pub struct QuicListener {
    server: Server,
}

impl QuicListener {
    pub async fn bind(addr: std::net::SocketAddr) -> anyhow::Result<Self> {
        let server = create_server(addr).await?;
        Ok(Self { server })
    }
}

#[async_trait]
impl BiListener for QuicListener {
    type Connection = QuicConnection;

    async fn accept(&mut self) -> anyhow::Result<Self::Connection> {
        let connection = self.server.accept().await.ok_or(anyhow::anyhow!("no connection"))?;
        Ok(QuicConnection { connection })
    }
}

// --- QuicConnect ---

pub struct QuicConnect {
    client: Client,
}

impl QuicConnect {
    pub async fn bind(addr: std::net::SocketAddr) -> anyhow::Result<Self> {
        let client = create_client(addr).await?;
        Ok(Self { client })
    }
}

#[async_trait]
impl BiConnect for QuicConnect {
    type Connection = QuicConnection;

    async fn connect(&mut self, addr: std::net::SocketAddr) -> anyhow::Result<Self::Connection> {
        let connection = self.client.connect(Connect::new(addr).with_server_name("localhost")).await?;
        Ok(QuicConnection { connection })
    }
}
