use crate::cpu::{combine_u8, split_u16};
use crate::mbc::{make_mbc, Mbc};
use std::pin::Pin;

pub struct Mmu {
    rom: Box<dyn Mbc + 'static>,
    wram_00: Pin<Vec<u8>>,
    wram_01: Pin<Vec<u8>>,
}

impl Mmu {
    pub fn new(fp: &str) -> Self {
        Self {
            rom: make_mbc(fp),
            wram_00: Pin::new(vec![0; 0x1000]),
            wram_01: Pin::new(vec![0; 0x1000]),
        }
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..0x8000 => self.rom.read_byte(addr),
            0xC000..0xD000 => self.wram_00[addr as usize - 0xC000],
            0xD000..0xE000 => self.wram_01[addr as usize - 0xD000],
            _ => todo!("UNSUPPORTED READ {:#06x}", addr),
        }
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        match addr {
            0x0000..0x8000 => self.rom.read_word(addr),
            0xC000..0xD000 => combine_u8(
                self.wram_00[addr as usize + 1 - 0xC000],
                self.wram_00[addr as usize - 0xC000],
            ),
            0xD000..0xE000 => combine_u8(
                self.wram_01[addr as usize + 1 - 0xD000],
                self.wram_01[addr as usize - 0xD000],
            ),
            _ => todo!("UNSUPPORTED READ {:#06x}", addr),
        }
    }

    pub fn write_byte(&mut self, addr: u16, v: u8) {
        match addr {
            0xC000..0xD000 => self.wram_00[addr as usize - 0xC000] = v,
            0xD000..0xE000 => self.wram_01[addr as usize - 0xD000] = v,
            _ => todo!("UNSUPPORTED WRITE {:#06x}", addr),
        }
    }

    pub fn write_word(&mut self, addr: u16, w: u16) {
        let (hi, lo) = split_u16(w);
        match addr {
            0xC000..0xD000 => {
                self.wram_00[addr as usize - 0xC000] = lo;
                self.wram_00[addr as usize + 1 - 0xC000] = hi;
            }
            0xD000..0xE000 => {
                self.wram_01[addr as usize - 0xD000] = lo;
                self.wram_01[addr as usize + 1 - 0xC000] = hi;
            }
            _ => todo!("UNSUPPORTED WRITE {:#06x}", addr),
        }
    }
}
