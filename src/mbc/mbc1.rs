use crate::mbc::{header_ram, header_rom, Mbc};

pub struct Mbc1 {
    rom: Vec<u8>,
    eram: Vec<u8>,
    mode: bool,
    rom_bank: usize,
    ram_bank: usize,
    ram_enable: bool,
    rom_banks: usize,
    ram_banks: usize,
    battery: bool,
}

impl Mbc for Mbc1 {
    fn boot(&self) {
        todo!()
    }

    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x0000..0x8000 => self.rom[self.translate(addr)],
            0xA000..0xC000 => self
                .ram_enable
                .then(|| self.eram[self.translate(addr)])
                .unwrap_or(0xFF),
            _ => todo!("UNSUPPORTED READ 0x{:04X}", addr),
        }
    }

    fn read_word(&self, addr: u16) -> u16 {
        todo!()
    }

    fn write_byte(&mut self, addr: u16, b: u8) {
        match addr {
            0x0000..0x2000 => self.ram_enable = b & 0x0F == 0x0A,
            0x2000..0x4000 => {
                // 5 BITS
                let b: usize = (b & 0b0001_1111) as usize;
                if b == 0 {
                    self.rom_bank = (self.rom_bank & 0b0110_0000) | 1; // preserve bits 5 and 6 !!
                } else {
                    let mask: usize = self.rom_banks - 1;
                    // e.g. 128 KiB cart (8 banks) => 0b0000_0111 (7)
                    self.rom_bank = (self.rom_bank & 0b0110_0000) | (b & mask);
                    // c.f. https://gbdev.io/pandocs/MBC1.html#20003fff--rom-bank-number-write-only
                }
            }
            0x4000..0x6000 => {
                // 2 BITS
                let b: usize = (b & 0b11) as usize;
                if self.rom_banks > 32 {
                    self.rom_bank = (self.rom_bank & 0x1F) | (b << 5);
                } else if self.ram_banks > 1 {
                    self.ram_bank = b;
                }
            }
            0x6000..0x8000 => self.mode = b & 1 == 1,
            0xA000..0xC000 => {
                if self.ram_enable {
                    let addr: usize = self.translate(addr);
                    self.eram[addr] = b;
                }
            }
            _ => todo!("UNSUPPORTED WRITE 0x{:04X}", addr),
        }
    }
}

impl Mbc1 {
    pub fn new(data: Vec<u8>) -> Self {
        let (battery, ram_banks) = match data[0x0147] {
            0x01 => (false, 0),
            0x02 => (false, header_ram(data[0x0149])),
            0x03 => (true, header_ram(data[0x0149])),
            _ => panic!(),
        };
        let rom_banks: usize = header_rom(data[0x0148]);
        Self {
            rom: data,
            eram: vec![0; 0x2000 * ram_banks],
            mode: false,
            rom_bank: 1,
            ram_bank: 0,
            ram_enable: false,
            rom_banks,
            ram_banks,
            battery,
        }
    }
    fn translate(&self, addr: u16) -> usize {
        match addr {
            0x0000..0x8000 => {
                let bank: usize = if addr < 0x4000 {
                    if !self.mode {
                        0
                    } else {
                        self.rom_bank & 0x60
                    }
                } else {
                    self.rom_bank
                };
                (addr as usize & 0x3FFF) + (bank * 0x4000)
            }
            0xA000..0xC000 => {
                let bank: usize = if self.mode { self.ram_bank } else { 0 };
                (addr as usize & 0x1FFF) + (bank * 0x2000)
            }
            _ => panic!(),
        }
    }
}
