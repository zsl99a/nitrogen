use std::error::Error;

use nitrogen::Nitrogen;
use nitrogen_extra::DiscoveryServiceKeepAlive;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let nitrogen = Nitrogen::new().await?.serve("0.0.0.0:0".parse()?).await?;

    println!("local_addr: {}", nitrogen.local_addr()?);
    println!("server_addr: {}", nitrogen.server_addr()?);

    loop {
        // master session
        let session = nitrogen.connect("127.0.0.1:31234".parse()?).await?;

        let keep_alive = DiscoveryServiceKeepAlive::keep_alive(nitrogen.clone(), session).await?;

        println!("keep_alived nodes: {:#?}", keep_alive.get_nodes().await?);

        tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)).await;
    }
}
