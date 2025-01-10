use crate::cpu::{Cpu, CpuTickOutput};
use std::io::{stdout, Write};
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::Duration;

pub mod cpu;
pub mod mbc;
pub mod mmu;
pub mod ppu;
mod utils;

pub const CLOCK_SPEED: u32 = 1048576;
pub const NANOS_PER_CYCLE: f64 = (1_000_000_000f64) / (CLOCK_SPEED as f64);

pub struct GbInput {}

pub struct GbOutput {
    pub frame: Vec<Vec<u8>>,
}

pub fn run_cpu(fp: &str) -> (SyncSender<GbInput>, Receiver<GbOutput>) {
    let mut cpu: Cpu = Cpu::boot(fp);
    let (gbin_tx, gbin_rx) = std::sync::mpsc::sync_channel(1);
    let (gbout_tx, gbout_rx) = std::sync::mpsc::sync_channel(1);

    std::thread::spawn(move || loop {
        let mut m_cycles = CLOCK_SPEED;
        while m_cycles > 0 {
            gbin_rx.recv().unwrap();
            let to: CpuTickOutput = cpu.tick();
            match to.sb {
                Some(c) => {
                    print!("{:}", c as char);
                    stdout().flush().unwrap();
                }
                None => (),
            };
            if to.draw {
                gbout_tx
                    .send(GbOutput {
                        frame: cpu.mmu.ppu.display_buffer.clone(),
                    })
                    .unwrap();
            }
            m_cycles = m_cycles.saturating_sub(to.m_cycles);
            let delay_nanos: u32 = (NANOS_PER_CYCLE * (to.m_cycles as f64)) as u32;
            std::thread::sleep(Duration::new(0, delay_nanos));
        }
    });
    (gbin_tx, gbout_rx)
}

pub fn timer(dur: Duration) -> Receiver<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || loop {
        std::thread::sleep(dur);
        match tx.send(()) {
            Err(_) => break,
            _ => (),
        }
    });
    rx
}
