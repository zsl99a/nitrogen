use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use nitrogen_utils::FramedTokioIO;
use parking_lot::Mutex;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::Session;

#[derive(Clone)]
pub struct Nitrogen2<S>
where
    S: AsyncRead + AsyncWrite + Send,
{
    services: Arc<Mutex<HashMap<String, Arc<dyn Service<S>>>>>,
}

impl<S> Nitrogen2<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    pub fn new() -> Self {
        Self { services: Default::default() }
    }
}

impl<S> Nitrogen2<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    pub fn add_service<Svc>(&mut self, service: Svc)
    where
        Svc: Service<S>,
    {
        self.services.lock().insert(service.name().to_string(), Arc::new(service));
    }

    pub fn service_list(&self) -> Vec<String> {
        self.services.lock().keys().cloned().collect()
    }

    pub fn get_service(&self, name: &str) -> Option<Arc<dyn Service<S>>> {
        self.services.lock().get(name).cloned()
    }
}

#[async_trait]
pub trait Service<S>
where
    S: AsyncRead + AsyncWrite + Send + 'static,
    Self: Send + Sync + 'static,
{
    fn name(&self) -> &'static str;

    fn initial(&self, nitrogen: Nitrogen2<S>);

    async fn serve(&self, framed_io: FramedTokioIO<S>, session: Session);
}
