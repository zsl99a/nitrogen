#[nitrogen::rpc_service]
pub trait SpeedTestingMainService {
    // master
    async fn dijkstra(&self);
    async fn upload(&self, ping_time: String);
}

#[nitrogen::rpc_service]
pub trait SpeedTestingNodeService {
    // node
    async fn ping(&self);
}
