use bam_maker_client::JitoMakerClient;

#[tokio::main]
async fn main() {
    let mut client = JitoMakerClient::new();
    client.sync_targets().await.expect("failed to sync targets");
    client
        .send_wire_transaction(&[0x00])
        .await
        .expect("failed to send wire transaction");
}
