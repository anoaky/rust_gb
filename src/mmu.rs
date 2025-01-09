use crate::constants::*;
use crate::cpu::{combine_u8, split_u16};
use crate::mbc::{make_mbc, Mbc};
use crate::ppu::Ppu;

pub struct Mmu {
    rom: Box<dyn Mbc + 'static>,
    pub ppu: Ppu,
    eram: Vec<u8>,
    wram: Vec<u8>,
}

impl Mmu {
    pub fn new(fp: &str) -> Self {
        Self {
            rom: make_mbc(fp),
            ppu: Ppu::new(),
            eram: vec![0; 0x2000],
            wram: vec![0; 0x2000],
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        let a16: usize = addr as usize;
        match addr {
            0x0000..0x8000 => self.rom.read_byte(addr),
            0x8000..0xA000 => self.ppu.read_byte(addr),
            0xA000..0xC000 => self.eram[a16 & 0x1FFF],
            0xC000..0xFE00 => self.wram[a16 & 0x1FFF],
            LY => self.ppu.read_byte(addr),
            _ => todo!("UNSUPPORTED READ 0x{:04X}", addr),
        }
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        combine_u8(self.read_byte(addr + 1), self.read_byte(addr))
    }

    pub fn write_byte(&mut self, addr: u16, v: u8) {
        let a16: usize = addr as usize;
        match addr {
            0x0000..0x8000 => self.rom.write_byte(addr, v),
            0x8000..0xA000 => self.ppu.write_byte(addr, v),
            0xA000..0xC000 => self.eram[a16 & 0x1FFF] = v,
            0xC000..0xFE00 => self.wram[a16 & 0x1FFF] = v,
            LY | LCDC | BGP => self.ppu.write_byte(addr, v),
            _ => todo!("UNSUPPORTED WRITE 0x{:04X}", addr),
        }
    }

    pub fn write_word(&mut self, addr: u16, w: u16) {
        let (hi, lo) = split_u16(w);
        self.write_byte(addr, lo);
        self.write_byte(addr + 1, hi);
    }
}
