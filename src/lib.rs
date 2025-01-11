use crate::cpu::{Cpu, CpuTickOutput};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

pub mod cpu;
pub mod mbc;
pub mod mmu;
pub mod ppu;
mod utils;

pub const CLOCK_SPEED: u32 = 1048576;
pub const NANOS_PER_CYCLE: f64 = (1_000_000_000f64) / (CLOCK_SPEED as f64);

pub struct GbInput {}

pub struct GbOutput {
    pub frame: [u8; 160 * 144],
}

pub fn run_cpu(fp: &str) -> (Sender<GbInput>, Receiver<Vec<u8>>) {
    let mut cpu = Box::new(Cpu::boot(fp));
    let (gbin_tx, gbin_rx) = std::sync::mpsc::channel();
    let (gbout_tx, gbout_rx) = std::sync::mpsc::sync_channel(1);
    let frame_timer = timer(Duration::new(0, 1_000_000_000u32 / 60));

    std::thread::spawn(move || 'cpu: loop {
        'draw: loop {
            match gbin_rx.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Empty) => (),
                Err(_) => break 'cpu,
            }
            let to: CpuTickOutput = cpu.tick();
            // match to.sb {
            //     Some(c) => {
            //         print!("{:}", c as char);
            //         stdout().flush().unwrap();
            //     }
            //     None => (),
            // };
            if to.draw {
                match gbout_tx.send(cpu.mmu.ppu.display_buffer.to_vec()) {
                    Ok(_) => break 'draw,
                    Err(_) => break 'cpu,
                }
            }
        }
        let _ = frame_timer.recv();
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

struct Stopwatch {
    instant: Instant,
}

impl Stopwatch {
    pub fn start() -> Self {
        Self {
            instant: Instant::now(),
        }
    }

    pub fn reset(&mut self) -> Duration {
        let elapsed = self.instant.elapsed();
        self.instant = Instant::now();
        return elapsed;
    }
}
