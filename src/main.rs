use rust_gb::{run_cpu, GbInput, GbOutput};
use std::io::{stdout, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (gbin_tx, gbout_rx) = run_cpu(&args[1]);
    gbin_tx.send(GbInput {}).unwrap();
    loop {
        let gbout: GbOutput = gbout_rx.recv().unwrap();
        if let Some(c) = gbout.sb {
            print!("{:}", c);
            stdout().flush().unwrap();
        }
        gbin_tx.send(GbInput {}).unwrap()
    }
}
