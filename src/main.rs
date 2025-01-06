use rust_gb::{run_cpu, GbOutput};
use std::sync::mpsc::Receiver;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let gbout_rx: Receiver<GbOutput> = run_cpu(&args[1]);
    loop {
        let output = gbout_rx.recv().unwrap();
        if output.serial_out {
            print!("{}", output.sb as char);
        }
    }
}
