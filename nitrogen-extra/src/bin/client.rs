use std::error::Error;

use nitrogen::Nitrogen;
use nitrogen_extra::{DiscoveryServiceClient, NodeInfo};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let nitrogen = Nitrogen::new().await?.serve("0.0.0.0:0".parse()?).await?;

    println!("local_addr: {}", nitrogen.local_addr()?);
    println!("server_addr: {}", nitrogen.server_addr()?);

    loop {
        let session = match nitrogen.connect("127.0.0.1:31234".parse()?).await {
            Ok(session) => session,
            Err(err) => {
                tracing::error!("nitrogen.connect error: {}", err);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let discovery_service = DiscoveryServiceClient::new(session.clone());
        let node_info = NodeInfo {
            server_addr: nitrogen.server_addr()?,
            services: nitrogen.services(),
        };
        if let Err(err) = discovery_service.register(node_info).await {
            tracing::error!("DiscoveryServiceClient::register error: {}", err);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        } else {
            println!("register success");
        }

        loop {
            match discovery_service.get_nodes().await {
                Ok(nodes) => {
                    println!("nodes: {:#?}\n", nodes);
                }
                Err(err) => {
                    tracing::error!("DiscoveryServiceClient::get_nodes error: {}", err);
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}
