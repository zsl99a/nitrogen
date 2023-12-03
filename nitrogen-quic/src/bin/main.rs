use futures::{SinkExt, StreamExt};
use nitrogen_quic::{create_server, BidirectionalStream};
use nitrogen_utils::{channel_sender_with_sink, framed_message_pack, FramedTokioIO};
use tokio_util::codec::LengthDelimitedCodec;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut server = create_server("0.0.0.0:31234".parse()?).await?;

    while let Some(mut connection) = server.accept().await {
        println!("connection id: {}", connection.id());

        tokio::spawn(async move {
            while let Ok(Some(bi_stream)) = connection.accept_bidirectional_stream().await {
                println!("stream id: {}", bi_stream.id());

                tokio::spawn(async move {
                    let framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(bi_stream);

                    handler(framed_io).await
                });
            }
        });
    }

    Ok(())
}

pub async fn handler(framed_io: FramedTokioIO<BidirectionalStream>) -> anyhow::Result<()> {
    let (sender, mut receiver) = framed_message_pack::<String, String, _>(framed_io).split();
    let mut sender = channel_sender_with_sink(sender);

    let ins = std::time::Instant::now();

    while let Some(Ok(msg)) = receiver.next().await {
        sender.send(msg).await?;
    }

    println!("recv instant: {:?}", ins.elapsed());

    Ok(())
}
