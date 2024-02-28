use solana_transfer_monitor::run;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}
