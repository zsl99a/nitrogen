use std::time::{Duration, Instant};

use futures::{SinkExt, StreamExt};
use nitrogen_quic::QuicConnect;
use nitrogen_utils::{channel_sender_with_sink, framed_message_pack, BiConnect, BiConnnectionOpener};
use tokio_util::codec::LengthDelimitedCodec;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = QuicConnect::bind("0.0.0.0:0".parse()?).await?;

    let mut connect = client.connect("127.0.0.1:31234".parse()?).await?;

    let bi_stream = connect.open().await?;

    let framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(bi_stream);

    let (sender, mut receiver) = framed_message_pack::<String, String, _>(framed_io).split();
    let mut sender = channel_sender_with_sink(sender);

    let ins = Instant::now();

    tokio::spawn(async move {
        for i in 0..100 {
            sender.send(format!("hello world {}", i)).await?;
        }
        anyhow::Result::<()>::Ok(())
    });

    while let Some(Ok(msg)) = receiver.next().await {
        println!("Instant: {:?},\t Recv: {}", ins.elapsed(), msg);
    }

    println!("recv instant: {:?}", ins.elapsed());

    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
