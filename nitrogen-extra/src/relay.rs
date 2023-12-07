use tokio::io::{copy_bidirectional, AsyncRead, AsyncWrite};

pub struct RelayService;

impl RelayService {
    pub async fn serve<I, O>(self, mut left: I, mut right: O)
    where
        I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        O: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        while let Err(err) = copy_bidirectional(&mut left, &mut right).await {
            tracing::error!("relay service error: {}", err);
        }
    }
}
