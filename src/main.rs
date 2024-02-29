use solana_transfer_monitor::run;

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}
