use futures::{channel::mpsc, StreamExt};

pub fn channel_sender_with_sink<T, S>(sink: S) -> mpsc::Sender<T>
where
    T: Send + 'static,
    S: futures::Sink<T> + Send + 'static,
    S::Error: Send,
{
    let (tx, rx) = mpsc::channel(128);
    tokio::spawn(rx.map(Ok).forward(sink));
    tx
}
