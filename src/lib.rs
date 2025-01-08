use crate::cpu::{Cpu, CpuTickOutput};
use anyhow::bail;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::time::Duration;

mod constants;
pub mod cpu;
pub mod mbc;
pub mod mmu;

pub const CLOCK_SPEED: u32 = 1048576;

pub struct GbOutput {
    pub sb: Option<char>,
}

pub struct GbInput {}

pub fn run_cpu(fp: &str) -> (SyncSender<GbInput>, Receiver<GbOutput>) {
    let mut cpu: Cpu = Cpu::new(fp);
    cpu.boot();
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
fn test_run(fp: &str) -> anyhow::Result<()> {
    let mut out: String = "".to_owned();
    let (gbin_tx, gbout_rx) = run_cpu(fp);
    gbin_tx.send(GbInput {})?;
    loop {
        let gbout: GbOutput = gbout_rx.recv()?;
        if let Some(c) = gbout.sb {
            out.push(c);
        }
        if out.ends_with("Passed") {
            return Ok(());
        } else if out.ends_with("Failed") {
            bail!("")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_run;

    #[test]
    fn blargg_01_cpu_special() -> anyhow::Result<()> {
        test_run("roms/01-special.gb")
    }

    #[test]
    fn blargg_02_interrupts() -> anyhow::Result<()> {
        test_run("roms/02-interrupts.gb")
    }

    #[test]
    fn blargg_03_cpu_op_sp_hl() -> anyhow::Result<()> {
        test_run("roms/03-op sp,hl.gb")
    }

    #[test]
    fn blargg_04_cpu_op_r_imm() -> anyhow::Result<()> {
        test_run("roms/04-op r,imm.gb")
    }

    #[test]
    fn blargg_05_cpu_op_rp() -> anyhow::Result<()> {
        test_run("roms/05-op rp.gb")
    }

    #[test]
    fn blargg_06_ld_r_r() -> anyhow::Result<()> {
        test_run("roms/06-ld r,r.gb")
    }

    #[test]
    fn blargg_07_subroutines() -> anyhow::Result<()> {
        test_run("roms/07-jr,jp,call,ret,rst.gb")
    }

    #[test]
    fn blargg_08_misc() -> anyhow::Result<()> {
        test_run("roms/08-misc instrs.gb")
    }

    #[test]
    fn blargg_09_op_r_r() -> anyhow::Result<()> {
        test_run("roms/09-op r,r.gb")
    }

    #[test]
    fn blargg_10_bit_ops() -> anyhow::Result<()> {
        test_run("roms/10-bit ops.gb")
    }

    #[test]
    fn blargg_11_op_a_hl() -> anyhow::Result<()> {
        test_run("roms/11-op a,(hl).gb")
    }
}
