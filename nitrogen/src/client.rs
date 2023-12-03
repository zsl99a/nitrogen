use std::collections::HashMap;

use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use nitrogen_utils::{channel_sender_with_sink, framed_message_pack};

use crate::{Error, Message, Negotiate, Result, Session};

#[async_trait::async_trait]
pub trait ServiceClient<Req: serde::Serialize + Send + 'static, Resp: serde::de::DeserializeOwned + Send + 'static> {
    const NAME: &'static str;

    fn tx(&self) -> mpsc::Sender<(Req, oneshot::Sender<Resp>)>;

    fn spawn(self, mut session: Session, mut rx: mpsc::Receiver<(Req, oneshot::Sender<Resp>)>) -> Self
    where
        Self: Sized,
    {
        tokio::spawn(async move {
            let framed_io = session.new_stream(Negotiate { name: Self::NAME.into() }).await?;

            let (sender, mut receiver) = framed_message_pack::<Message<Resp>, Message<Req>, _>(framed_io).split();
            let mut sender = channel_sender_with_sink(sender);

            let mut cursor = 0;
            let mut notifies = HashMap::new();

            loop {
                tokio::select! {
                    Some((req, notify)) = rx.next() => {
                        cursor += 1;
                        let id = cursor;
                        if let Err(err) = sender.send(Message { id, payload: req }).await {
                            tracing::error!("{}Client::request send error: {}", Self::NAME, err);
                            break;
                        } else {
                            notifies.insert(id, notify);
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

            anyhow::Result::<()>::Ok(())
        });

        self
    }

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
