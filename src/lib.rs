use crate::cpu::Cpu;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TrySendError;
use std::time::Duration;

pub mod cpu;
pub mod mbc;
pub mod mmu;

pub const CLOCK_SPEED: u32 = 4194304;

pub struct GbOutput {
    pub serial_out: bool,
    pub sb: u8,
}

pub fn run_cpu(fp: &str) -> Receiver<GbOutput> {
    let mut cpu: Cpu = Cpu::new(fp);
    let (gbout_tx, gbout_rx) = std::sync::mpsc::sync_channel(1);
    let timer_rx: Receiver<()> = timer();
    std::thread::spawn(move || loop {
        let mut m_cycles: u32 = 0;
        while m_cycles < CLOCK_SPEED {
            m_cycles += cpu.tick().m_cycles;
        }
        let gbout = GbOutput {
            serial_out: cpu.sc_enable,
            sb: cpu.sb,
        };
        match gbout_tx.try_send(gbout) {
            Err(TrySendError::Disconnected(_)) => break,
            _ => (),
        }
        if cpu.sc_enable {
            cpu.sc_enable = false;
        }
        timer_rx.recv().unwrap();
    });
    gbout_rx
}

pub fn timer() -> Receiver<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        match tx.try_send(()) {
            Err(TrySendError::Disconnected(_)) => break,
            _ => (),
        }
    });
    rx
}
