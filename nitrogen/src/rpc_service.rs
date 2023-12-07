use std::collections::HashMap;

use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use nitrogen_utils::{channel_sender_with_sink, framed_message_pack};
use serde::{Deserialize, Serialize};
use tokio_util::codec::LengthDelimitedCodec;

// --- Message ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<T> {
    pub id: u64,
    pub payload: T,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error(pub String);

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Nitrogen Error: {}", self.0)
    }
}

// RpcServiceClient 通过 rpc_service 自动实现

#[async_trait::async_trait]
pub trait RpcServiceClient<Req, Resp>
where
    Req: serde::Serialize + Send + 'static,
    Resp: serde::de::DeserializeOwned + Send + 'static,
{
    const NAME: &'static str;

    fn tx(&self) -> mpsc::Sender<(Req, oneshot::Sender<Resp>)>;

    #[doc(hidden)]
    fn spawn<S>(self, mut rx: mpsc::Receiver<(Req, oneshot::Sender<Resp>)>, stream: S) -> Self
    where
        Self: Sized,
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + 'static,
    {
        tokio::spawn(async move {
            let framed_io = LengthDelimitedCodec::builder().max_frame_length(1024 * 1024 * 16).new_framed(stream);

            let (sender, mut receiver) = framed_message_pack::<Message<Resp>, Message<Req>, _>(framed_io).split();
            let mut sender = channel_sender_with_sink(sender);

            let mut cursor = 0;
            let mut notifies = HashMap::new();

            loop {
                tokio::select! {
                    write = rx.next() => {
                        if let Some((payload, notify)) = write {
                            cursor += 1;
                            let id = cursor;
                            if let Err(err) = sender.send(Message { id, payload }).await {
                                tracing::error!("{}Client::request send error: {}", Self::NAME, err);
                            } else {
                                notifies.insert(id, notify);
                            }
                        } else {
                            break;
                        }
                    }
                    Some(result) = receiver.next() => {
                        match result {
                            Ok(Message { id, payload }) => {
                                if let Some(notify) = notifies.remove(&id) {
                                    let _ = notify.send(payload);
                                }
                            }
                            Err(err) => {
                                tracing::error!("{}Client::request recv error: {}", Self::NAME, err);
                            }
                        }
                    }
                }
            }

            anyhow::Result::<()>::Ok(())
        });

        self
    }

    #[doc(hidden)]
    async fn request(&self, req: Req) -> Result<Resp> {
        let (tx, rx) = oneshot::channel::<Resp>();
        self.tx()
            .send((req, tx))
            .await
            .map_err(|err| Error(format!("{}Client::request send error: {}", Self::NAME, err)))?;
        match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
            Ok(Ok(res)) => Ok(res),
            Ok(Err(err)) => Err(Error(format!("{}Client::request recv error: {}", Self::NAME, err))),
            Err(err) => Err(Error(format!("{}Client::request timeout error: {}", Self::NAME, err))),
        }
    }
}
