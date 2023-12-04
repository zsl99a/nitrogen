use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_serde::formats::MessagePack;
use tokio_util::codec::LengthDelimitedCodec;

pub type FramedTokioIO<S> = tokio_util::codec::Framed<S, LengthDelimitedCodec>;

pub type FramedMessagePack<Item, SinkItem, S> = tokio_serde::Framed<FramedTokioIO<S>, Item, SinkItem, MessagePack<Item, SinkItem>>;

pub fn framed_message_pack<Item, SinkItem, S>(framed_io: FramedTokioIO<S>) -> FramedMessagePack<Item, SinkItem, S>
where
    Item: DeserializeOwned + Send + 'static,
    SinkItem: Serialize + Send + 'static,
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    tokio_serde::Framed::new(framed_io, MessagePack::default())
}
