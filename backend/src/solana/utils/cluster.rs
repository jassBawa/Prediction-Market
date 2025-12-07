use anchor_client::Cluster;

pub fn detect_cluster(rpc_url: &str) -> Cluster {
    if rpc_url.contains("localhost") || rpc_url.contains("127.0.0.1") {
        Cluster::Localnet
    } else if rpc_url.contains("devnet") {
        Cluster::Devnet
    } else if rpc_url.contains("mainnet") {
        Cluster::Mainnet
    } else {
        Cluster::Custom(rpc_url.to_string(), rpc_url.to_string())
    }
}
