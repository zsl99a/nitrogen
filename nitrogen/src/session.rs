use std::ops::Deref;

use anyhow::Result;
use futures::SinkExt;
use nitrogen_quic::{QuicConnectionOpener, QuicStream};
use nitrogen_utils::{BiConnnectionOpener, FramedTokioIO};
use tokio_util::codec::LengthDelimitedCodec;

use crate::model::Negotiate;

#[derive(Clone)]
pub struct Session {
    opener: QuicConnectionOpener,
}

impl Deref for Session {
    type Target = QuicConnectionOpener;

    fn deref(&self) -> &Self::Target {
        &self.opener
    }
}

impl Session {
    pub fn new(opener: QuicConnectionOpener) -> Self {
        Self { opener }
    }

    pub async fn new_stream(&mut self, negotiate: Negotiate) -> Result<FramedTokioIO<QuicStream>> {
        let bi_stream = self.opener.open().await?;
        let mut framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(bi_stream);
        framed_io.send(rmp_serde::to_vec(&negotiate)?.into()).await?;
        Ok(framed_io)
    }
}
