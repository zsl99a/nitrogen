use std::error::Error;

use nitrogen::Nitrogen;
use nitrogen_extra::{DiscoveryService, DiscoveryServiceExt, DiscoveryServiceImpl, DiscoveryServiceStore, NodeInfo};

// master node server
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let nodes = DiscoveryServiceStore::default();

    let nitrogen = Nitrogen::new()
        .await?
        .add_service(DiscoveryServiceImpl::NAME, {
            let nodes = nodes.clone();
            move |framed_io, session, _nitrogen| DiscoveryServiceImpl::new(session, nodes.clone()).serve(framed_io)
        })
        .serve("0.0.0.0:31234".parse()?)
        .await?;

    nodes.lock().insert(
        u64::MAX,
        NodeInfo {
            server_addr: nitrogen.server_addr()?,
            services: nitrogen.services(),
        },
    );

    // let session = nitrogen.connect("127.0.0.1:31234".parse()?).await?;
    // let s = DiscoveryServiceClient::new(session.clone());
    // s.get_nodes().await?;

    loop {
        println!("nodes: {:#?}\n", nodes.lock());
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
