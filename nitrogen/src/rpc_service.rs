use std::collections::HashMap;

use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use nitrogen_utils::{channel_sender_with_sink, framed_message_pack};

use crate::{Error, Message, Negotiate, Result, Session};

// RpcServiceClient 通过 rpc_service 自动实现

#[async_trait::async_trait]
pub trait RpcServiceClient<Req, Resp>
where
    Req: serde::Serialize + Send + 'static,
    Resp: serde::de::DeserializeOwned + Send + 'static,
{
    const NAME: &'static str;

    fn tx(&self) -> mpsc::Sender<(Req, oneshot::Sender<Resp>)>;

    fn session(&self) -> &Session;

    #[doc(hidden)]
    fn spawn(self, mut rx: mpsc::Receiver<(Req, oneshot::Sender<Resp>)>) -> Self
    where
        Self: Sized,
    {
        let mut session = self.session().clone();

        tokio::spawn(async move {
            loop {
                let framed_io = match session.new_stream(Negotiate { name: Self::NAME.into() }).await {
                    Ok(framed_io) => framed_io,
                    Err(err) => {
                        tracing::error!("{}Client::new_stream error: {}", Self::NAME, err);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let (sender, mut receiver) = framed_message_pack::<Message<Resp>, Message<Req>, _>(framed_io).split();
                let mut sender = channel_sender_with_sink(sender);

                let mut cursor = 0;
                let mut notifies = HashMap::new();

                loop {
                    tokio::select! {
                        write = rx.next() => {
                            if let Some((payload, notify)) = write {
                                println!("{}Client::request: {}", Self::NAME, cursor);

                                cursor += 1;
                                let id = cursor;
                                if let Err(err) = sender.send(Message { id, payload }).await {
                                    tracing::error!("{}Client::request send error: {}", Self::NAME, err);
                                    break;
                                } else {
                                    notifies.insert(id, notify);
                                }
                            } else {
                                return;
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
                                    break;
                                }
                            }
                        }
                        else => break,
                    }
                }
            }
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
