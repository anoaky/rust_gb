use crate::cpu::{combine_u8, split_u16};
use crate::mbc::{make_mbc, Mbc};
use crate::ppu::Ppu;
use crate::utils::*;

pub struct Mmu {
    cart: Box<dyn Mbc + 'static>,
    pub ppu: Ppu,
    wram: Vec<u8>,
}

impl Mmu {
    pub fn new(fp: &str) -> Self {
        Self {
            cart: make_mbc(fp),
            ppu: Ppu::new(),
            wram: vec![0; 0x2000],
        }
    }

    pub fn boot(fp: &str) -> Self {
        Self {
            ppu: Ppu::boot(),
            ..Self::new(fp)
        }
    }

    pub fn cycle(&mut self, cycles: u16) {
        self.ppu.cycle(cycles);
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        let a16: usize = addr as usize;
        match addr {
            0x0000..0x8000 => self.cart.read_byte(addr),
            0x8000..0xA000 => self.ppu.read_byte(addr),
            0xA000..0xC000 => self.cart.read_byte(addr),
            0xC000..0xFE00 => self.wram[a16 & 0x1FFF],
            LCDC..=WX => self.ppu.read_byte(addr),
            0xFF4D => 0xFF,
            _ => todo!("UNSUPPORTED READ 0x{:04X}", addr),
        }
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        combine_u8(self.read_byte(addr + 1), self.read_byte(addr))
    }

    pub fn write_byte(&mut self, addr: u16, v: u8) {
        let a16: usize = addr as usize;
        match addr {
            0x0000..0x8000 => self.cart.write_byte(addr, v),
            0x8000..0xA000 => self.ppu.write_byte(addr, v),
            0xA000..0xC000 => self.cart.write_byte(addr, v),
            0xC000..0xFE00 => self.wram[a16 & 0x1FFF] = v,
            LCDC..=WX => self.ppu.write_byte(addr, v),
            0xFF4D => (),
            _ => todo!("UNSUPPORTED WRITE 0x{:04X}", addr),
        }
    }

    pub fn write_word(&mut self, addr: u16, w: u16) {
        let (hi, lo) = split_u16(w);
        self.write_byte(addr, lo);
        self.write_byte(addr + 1, hi);
    }
}
