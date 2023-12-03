use std::net::SocketAddr;

use anyhow::Result;
use futures::SinkExt;
use nitrogen_quic::{BidirectionalStream, Handle};
use nitrogen_utils::FramedTokioIO;
use tokio_util::codec::LengthDelimitedCodec;

use crate::model::Negotiate;

#[derive(Debug, Clone)]
pub struct Session {
    handle: Handle,
}

impl Session {
    pub fn new(handle: Handle) -> Self {
        Self { handle }
    }

    pub async fn new_stream(&mut self, negotiate: Negotiate) -> Result<FramedTokioIO<BidirectionalStream>> {
        let bi_stream = self.handle.open_bidirectional_stream().await?;
        let mut framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(bi_stream);
        framed_io.send(rmp_serde::to_vec(&negotiate)?.into()).await?;
        Ok(framed_io)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.handle.local_addr()?)
    }

    pub fn remote_addr(&self) -> Result<SocketAddr> {
        Ok(self.handle.remote_addr()?)
    }
}

impl Session {}
