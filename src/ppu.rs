use crate::ppu::PpuMode::*;
use crate::utils::*;

pub struct Ppu {
    vram: Vec<u8>,
    oam: Vec<u8>,
    ppu_mode: PpuMode,
    pub ly: u8,
    lyc: u8,
    scy: u8,
    scx: u8,
    wx: u8,
    wy: u8,
    bgp: u8,
    stat: u8,
    lcdc: u8,
    obp0: u8,
    obp1: u8,
    pub display_buffer: [u8; 160 * 144],
    dots: u16,
    pub dma: bool,
    pub dma_src: u16,
    pub int_line: bool,
    pub vblank: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: vec![0; 0x2000],
            oam: vec![0; 0xA0],
            ppu_mode: Mode2,
            ly: 0,
            lyc: 0,
            scy: 0,
            scx: 0,
            bgp: 0,
            wx: 0,
            wy: 0,
            obp0: 0,
            obp1: 0,
            lcdc: 0,
            stat: 0,
            display_buffer: [0; 160 * 144],
            dots: 0,
            dma: false,
            dma_src: 0,
            int_line: false,
            vblank: false,
        }
    }

    pub fn boot() -> Self {
        Self {
            bgp: 0xFC,
            stat: 0x85,
            lcdc: 0x91,
            ..Self::new()
        }
    }

    pub fn cycle(&mut self, cycles: u16) {
        self.vblank = false;
        if !bit(self.lcdc, 7) {
            return;
        }
        let mut available_dots = cycles * 4;

        while available_dots > 0 {
            let dots = if available_dots >= 80 {
                80
            } else {
                available_dots
            };
            self.dots += dots;
            available_dots -= dots;

            if self.dots >= 456 {
                self.dots -= 456;
                self.ly = (self.ly + 1) % 154;
                if self.ly == self.lyc {
                    // println!("LYC==LY");
                    self.stat |= 4;
                    // println!("STAT: 0b{:08b}", self.stat);
                } else {
                    self.stat &= !4;
                }
                if self.ly == 144 && self.ppu_mode != Mode1 {
                    self.vblank = true;
                    self.ppu_mode = Mode1;
                }
            }

            if self.ly < 144 {
                if self.dots <= 80 {
                    if self.ppu_mode != Mode2 {
                        self.ppu_mode = Mode2;
                    }
                } else if self.dots <= 80 + 172 {
                    if self.ppu_mode != Mode3 {
                        self.ppu_mode = Mode3;
                        self.draw_bg();
                    }
                } else {
                    if self.ppu_mode != Mode0 {
                        self.ppu_mode = Mode0;
                    }
                }
            }
            self.stat = (self.stat & 0b1111_1100) | (self.ppu_mode as u8);
            self.set_intline();
        }
    }

    pub fn dma_transfer(&mut self, byte: u8, offset: u8) {
        // println!("DMA TRANSFER TO 0x{:04X}", 0xFE00 | offset as u16);
        self.oam[offset as usize] = byte;
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x8000..0xA000 => {
                if self.ppu_mode == Mode3 && bit(self.lcdc, 7) {
                    0xFF
                } else {
                    self.read_vram(addr)
                }
            }
            0xFE00..0xFEA0 => {
                if bit(self.lcdc, 7) && (self.ppu_mode == Mode2 || self.ppu_mode == Mode3) {
                    0xFF
                } else {
                    self.oam[(addr & 0xFF) as usize]
                }
            }
            0xFEA0..0xFF00 => {
                // todo OAM corruption
                if self.dma {
                    0xFF
                } else {
                    0x00
                }
            }
            BGP => self.bgp,
            LY => self.ly,
            LCDC => self.lcdc,
            LYC => self.lyc,
            DMA => (self.dma_src >> 8) as u8,
            SCY => self.scy,
            SCX => self.scx,
            WY => self.wy,
            WX => self.wx,
            STAT => self.stat,
            OBP0 => self.obp0,
            OBP1 => self.obp1,
            _ => todo!("UNSUPPORTED READ 0x{:04X}", addr),
        }
    }

    pub fn write_byte(&mut self, addr: u16, b: u8) {
        match addr {
            0x8000..0xA000 => {
                if bit(self.lcdc, 7) && self.ppu_mode == Mode3 {
                    // println!("FAILED TO WRITE VRAM! MODE {:?}", self.ppu_mode);
                } else {
                    self.write_vram(addr, b);
                }
            }
            0xFE00..0xFEA0 => {
                if bit(self.lcdc, 7) && (self.ppu_mode == Mode2 || self.ppu_mode == Mode3) {
                    ()
                } else {
                    self.oam[(addr & 0xFF) as usize] = b;
                }
            }
            0xFEA0..0xFF00 => (),
            LCDC => {
                // println!("SET LCDC: 0b{:08b}", b);
                self.lcdc = b
            }
            BGP => self.bgp = b,
            LY => (),
            LYC => {
                // println!("SET LYC {}", b);
                self.lyc = b
            }
            DMA => {
                self.dma_src = (b as u16) << 8;
                self.dma = true;
            }
            SCY => {
                // println!("SET SCY {}", b);
                self.scy = b
            }
            SCX => {
                // println!("SET SCX {}", b);
                self.scx = b
            }
            WY => self.wy = b,
            WX => self.wx = b,
            STAT => self.stat = b,
            OBP0 => self.obp0 = b,
            OBP1 => self.obp1 = b,
            _ => todo!("UNSUPPORTED WRITE 0x{:04X}", addr),
        }
    }

    fn draw_bg(&mut self) {
        let scy: u8 = self.scy.wrapping_add(self.ly);
        let tiley: u16 = (scy as u16 / 8) % 32;
        for lx in 0..160u8 {
            let scx: u8 = self.scx.wrapping_add(lx);
            let tilex: u16 = (scx as u16 / 8) % 32;
            let py: u8 = scy % 8;
            let px: u8 = scx % 8;
            let tmap_base: u16 = 0x9800 | (bit(self.lcdc, 3) as u16) << 10;
            let tile_id: u8 = self.read_vram(tmap_base + tiley * 0x20 + tilex);
            let tile_addr: u16 = if bit(self.lcdc, 4) {
                0x8000 + 0x10 * tile_id as u16
            } else {
                0x9000 + 0x10 * (tile_id as i8 as i16 + 128) as u16
            };
            let tile_lo: u8 = self.read_vram(tile_addr + 2 * py as u16);
            let tile_hi: u8 = self.read_vram(tile_addr + 2 * py as u16 + 1);
            let colour_lsb = bit(tile_lo, 7 - px);
            let colour_msb = bit(tile_hi, 7 - px);
            let colour: u8 = match (colour_msb, colour_lsb) {
                (false, false) => self.bgp & 3,
                (false, true) => (self.bgp >> 2) & 3,
                (true, false) => (self.bgp >> 4) & 3,
                (true, true) => (self.bgp >> 6) & 3,
            };
            self.display_buffer[self.ly as usize * 160 + lx as usize] = colour;
        }
    }

    fn read_vram(&self, addr: u16) -> u8 {
        self.vram[addr as usize - 0x8000]
    }

    fn set_intline(&mut self) {
        let lyc: bool = bit(self.stat, 6) && bit(self.stat, 2);
        let mode2: bool = bit(self.stat, 5) && self.ppu_mode == Mode2;
        let mode1: bool = bit(self.stat, 4) && self.ppu_mode == Mode1;
        let mode0: bool = bit(self.stat, 3) && self.ppu_mode == Mode0;
        self.int_line = lyc || mode2 || mode1 || mode0;
    }

    fn write_vram(&mut self, addr: u16, b: u8) {
        self.vram[addr as usize - 0x8000] = b;
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum PpuMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}
