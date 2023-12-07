use bytes::{BufMut, BytesMut};
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct Negotiator<N>
where
    N: Serialize + DeserializeOwned + Send + 'static,
{
    _marker: std::marker::PhantomData<N>,
}

impl<N> Negotiator<N>
where
    N: Serialize + DeserializeOwned + Send + 'static,
{
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<N> Negotiator<N>
where
    N: Serialize + DeserializeOwned + Send + 'static,
{
    pub async fn recv<I>(&mut self, io: &mut I) -> anyhow::Result<N>
    where
        I: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let mut buf = [0u8; 2];
        io.read_exact(&mut buf).await?;
        let len = u16::from_be_bytes(buf) as usize;
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        Ok(rmp_serde::from_slice(&buf)?)
    }

    pub async fn send<I>(&mut self, io: &mut I, msg: N) -> anyhow::Result<()>
    where
        I: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let msg = bytes::Bytes::from(rmp_serde::to_vec(&msg)?);
        let mut buf = BytesMut::new();
        buf.put_u16(msg.len() as u16);
        buf.put(msg);
        io.write_all(&buf).await?;
        Ok(())
    }
}
