use std::{collections::HashMap, net::SocketAddr, ops::Deref, sync::Arc};

use nitrogen::{Nitrogen, Session};
use parking_lot::Mutex;

type NodeInfoMap = HashMap<u64, NodeInfo>;

#[derive(Default, Clone)]
pub struct DiscoveryServiceStore(Arc<Mutex<NodeInfoMap>>);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeInfo {
    pub server_addr: SocketAddr,
    pub services: Vec<String>,
}

impl Deref for DiscoveryServiceStore {
    type Target = Arc<Mutex<NodeInfoMap>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[nitrogen::rpc_service]
pub trait DiscoveryService {
    async fn register(&self, node_info: NodeInfo);

    async fn get_nodes(&self) -> NodeInfoMap;
}

#[derive(Clone)]
pub struct DiscoveryServiceImpl {
    session: Session,
    nodes: DiscoveryServiceStore,
    strong: Arc<()>,
}

impl DiscoveryServiceImpl {
    pub fn new(session: Session, nodes: DiscoveryServiceStore) -> Self {
        Self {
            session,
            nodes,
            strong: Arc::new(()),
        }
    }
}

#[async_trait::async_trait]
impl DiscoveryService for DiscoveryServiceImpl {
    async fn register(&self, node_info: NodeInfo) {
        self.nodes.lock().insert(self.session.id(), node_info);
    }

    async fn get_nodes(&self) -> NodeInfoMap {
        self.nodes.lock().clone()
    }
}

impl Drop for DiscoveryServiceImpl {
    fn drop(&mut self) {
        if Arc::strong_count(&self.strong) > 1 {
            return;
        }
        self.nodes.lock().remove(&self.session.id());
    }
}

pub struct DiscoveryServiceKeepAlive(DiscoveryServiceClient);

impl Deref for DiscoveryServiceKeepAlive {
    type Target = DiscoveryServiceClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DiscoveryServiceKeepAlive {
    pub async fn keep_alive(nitrogen: Nitrogen, session: Session) -> anyhow::Result<Self> {
        let client = DiscoveryServiceClient::new(session);

        tokio::spawn({
            let client = client.clone();
            async move {
                loop {
                    if let Err(err) = client
                        .register(NodeInfo {
                            server_addr: nitrogen.server_addr().expect("no server_addr"),
                            services: nitrogen.services().clone(),
                        })
                        .await
                    {
                        tracing::error!("DiscoveryServiceClient::register error: {}", err);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        });

        Ok(Self(client))
    }
}
