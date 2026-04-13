use bam_maker_client::JitoMakerClient;

fn main() {
    let mut client = JitoMakerClient::new();
    client.sync_targets().expect("failed to sync targets");
    client
        .send_wire_transaction(&[0x00])
        .expect("failed to send wire transaction");
}
