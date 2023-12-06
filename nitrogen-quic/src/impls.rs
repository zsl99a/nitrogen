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

pub struct QuicStream {
    stream: BidirectionalStream,
}

impl tokio::io::AsyncRead for QuicStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for QuicStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}

impl BiStreamSplit for QuicStream {
    type Read = ReceiveStream;
    type Write = SendStream;

    fn split(self) -> (Self::Read, Self::Write) {
        let (recv, send) = self.stream.split();
        (recv, send)
    }
}

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
    type Acceptor = QuicConnectionAcceptor;
    type Opener = QuicConnectionOpener;

    fn split(self) -> (Self::Acceptor, Self::Opener) {
        let (opener, acceptor) = self.connection.split();
        (QuicConnectionAcceptor { acceptor }, QuicConnectionOpener { opener })
    }
}

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

pub struct QuicListener {
    listener: Server,
}

impl QuicListener {
    pub async fn bind(addr: std::net::SocketAddr) -> anyhow::Result<Self> {
        let listener = create_server(addr).await?;
        Ok(Self { listener })
    }
}

#[async_trait]
impl BiListener for QuicListener {
    type Connection = QuicConnection;

    async fn accept(&mut self) -> anyhow::Result<Self::Connection> {
        let connection = self.listener.accept().await.ok_or(anyhow::anyhow!("no connection"))?;
        Ok(QuicConnection { connection })
    }
}

pub struct QuicConnect {
    connect: Client,
}

impl QuicConnect {
    pub async fn bind(addr: std::net::SocketAddr) -> anyhow::Result<Self> {
        let connect = create_client(addr).await?;
        Ok(Self { connect })
    }
}

#[async_trait]
impl BiConnect for QuicConnect {
    type Connection = QuicConnection;

    async fn connect(&mut self, addr: std::net::SocketAddr) -> anyhow::Result<Self::Connection> {
        let connection = self.connect.connect(Connect::new(addr).with_server_name("localhost")).await?;
        Ok(QuicConnection { connection })
    }
}
