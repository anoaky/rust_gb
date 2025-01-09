use crate::cpu::{Cpu, CpuTickOutput};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::time::Duration;

mod constants;
pub mod cpu;
pub mod mbc;
pub mod mmu;
pub mod ppu;

pub const CLOCK_SPEED: u32 = 1048576;

pub struct GbOutput {
    pub sb: Option<char>,
}

pub struct GbInput {}

pub fn run_cpu(fp: &str) -> (SyncSender<GbInput>, Receiver<GbOutput>) {
    let mut cpu: Cpu = Cpu::new(fp);
    cpu.boot_dmg();
    let (gbout_tx, gbout_rx) = std::sync::mpsc::sync_channel(1);
    let (gbin_tx, gbin_rx) = std::sync::mpsc::sync_channel(1);
    let timer_rx: Receiver<()> = timer();
    std::thread::spawn(move || loop {
        let mut m_cycles: u32 = 0;
        while m_cycles < CLOCK_SPEED {
            match gbin_rx.try_recv() {
                Err(TryRecvError::Disconnected) => break,
                _ => (),
            }
            let to: CpuTickOutput = cpu.tick();
            m_cycles += to.m_cycles;
            let sb: Option<char> = match to.sb {
                Some(c) => Some(c as char),
                None => None,
            };
            gbout_tx.send(GbOutput { sb }).unwrap();
        }
        timer_rx.recv().unwrap();
    });
    (gbin_tx, gbout_rx)
}

pub fn timer() -> Receiver<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        match tx.send(()) {
            Err(_) => break,
            _ => (),
        }
    });
    rx
}

#[cfg(test)]
fn test_run(fp: &str) -> anyhow::Result<String> {
    use anyhow::bail;

    let mut out: String = "".to_owned();
    let (gbin_tx, gbout_rx) = run_cpu(fp);
    gbin_tx.send(GbInput {})?;
    loop {
        let gbout: GbOutput = gbout_rx.recv()?;
        if let Some(c) = gbout.sb {
            out.push(c);
        }
        if out.ends_with("Passed") {
            return Ok(out);
        } else if out.ends_with("Failed") {
            bail!(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{run_cpu, GbInput, GbOutput};
    use anyhow::bail;

    fn test_agg(fp: &str) -> anyhow::Result<()> {
        // for blargg aggregate roms (e.g., cpu_instrs)

        let mut test_codes: Vec<String> = Vec::new();
        let mut output: String = "".to_owned();
        let (gbin_tx, gbout_rx) = run_cpu(fp);
        gbin_tx.send(GbInput {})?;
        loop {
            let gbout: GbOutput = gbout_rx.recv()?;
            if let Some(c) = gbout.sb {
                output.push(c);
            }
            output = output.trim_ascii().to_string();
            match output.as_bytes() {
                [.., b':', b'o', b'k'] => {
                    test_codes.push(output.clone());
                    output = "".to_owned();
                }
                [.., t1 @ b'0'..=b'9', t2 @ b'0'..=b'9', b':', b'0'..=b'9', b'0'..=b'9'] => {
                    bail!("Failed {}{}", *t1 as char, *t2 as char);
                }
                _ => (),
            }

            if output.ends_with("Passed") || test_codes.len() >= 11 {
                return Ok(());
            } else if output.ends_with("Failed") {
                bail!("Failed!");
            }
        }
    }

    #[test]
    fn blargg_cpu_instrs() -> anyhow::Result<()> {
        test_agg("roms/cpu_instrs/cpu_instrs.gb")
    }
}
