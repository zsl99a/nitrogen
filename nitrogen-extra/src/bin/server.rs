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
            move |framed_io, session, _nitrogen| {
                let nodes = nodes.clone();
                async move {
                    DiscoveryServiceImpl::new(session, nodes.clone()).serve(framed_io).await;
                    println!("DiscoveryServiceImpl::serve exit");
                }
            }
        })
        .serve("0.0.0.0:31234".parse()?)
        .await?;

    nodes.lock().insert(
        u64::MAX,
        NodeInfo {
            server_addr: nitrogen.server_addr()?,
            services: nitrogen.services().clone(),
        },
    );

    loop {
        println!("nodes: {:#?}\n", nodes.lock());
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
