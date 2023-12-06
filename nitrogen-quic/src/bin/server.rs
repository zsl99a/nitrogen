use futures::{SinkExt, StreamExt};
use nitrogen_quic::QuicListener;
use nitrogen_utils::{channel_sender_with_sink, framed_message_pack, BiConnnectionAcceptor, BiListener, FramedTokioIO};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::LengthDelimitedCodec;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut server = QuicListener::bind("0.0.0.0:31234".parse()?).await?;

    while let Ok(mut connection) = server.accept().await {
        tokio::spawn(async move {
            while let Ok(bi_stream) = connection.accept().await {
                tokio::spawn(async move {
                    let framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(bi_stream);

                    handler(framed_io).await
                });
            }
        });
    }

    Ok(())
}

pub async fn handler<S>(framed_io: FramedTokioIO<S>) -> anyhow::Result<()>
where
    S: AsyncWrite + AsyncRead + Send + 'static,
{
    let (sender, mut receiver) = framed_message_pack::<String, String, _>(framed_io).split();
    let mut sender = channel_sender_with_sink(sender);

    let ins = std::time::Instant::now();

    while let Some(Ok(msg)) = receiver.next().await {
        sender.send(msg).await?;
    }

    println!("recv instant: {:?}", ins.elapsed());

    Ok(())
}
