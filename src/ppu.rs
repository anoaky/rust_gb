use crate::ppu::PpuMode::*;
use crate::utils::*;
use std::collections::VecDeque;

pub struct Ppu {
    vram: Vec<u8>,
    ppu_mode: PpuMode,
    pub ly: u8,
    lyc: u8,
    scy: u8,
    scx: u8,
    wx: u8,
    wy: u8,
    bgp: u8,
    lx: u8,
    stat: u8,
    lcdc: u8,
    pub display_buffer: Vec<Vec<u8>>,
    bg_fifo: VecDeque<FIFOPixel>,
    available_dots: u16,
    dots: u16,
    pub vblank: bool,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: vec![0; 0x2000],
            ppu_mode: Mode2,
            ly: 0,
            lyc: 0,
            scy: 0,
            scx: 0,
            bgp: 0,
            wx: 0,
            wy: 7,
            lx: 0,
            lcdc: 0,
            stat: 0,
            display_buffer: vec![vec![0; 160]; 144],
            bg_fifo: VecDeque::with_capacity(16),
            available_dots: 0,
            dots: 0,
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
                    self.stat = set(self.stat, 2) as u8;
                }
                if self.ly >= 144 && self.ppu_mode != Mode1 {
                    self.ppu_mode = Mode1;
                    self.vblank = true;
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
                    }
                } else {
                    if self.ppu_mode != Mode0 {
                        self.ppu_mode = Mode0;
                        self.draw_bg();
                    }
                }
            }
        }
        self.stat = (self.stat & 0b1111_1100) | (self.ppu_mode as u8);
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0x8000..0xA000 => {
                if self.ppu_mode != Mode3 {
                    self.read_vram(addr)
                } else {
                    0xFF
                }
            }
            BGP => self.bgp,
            LY => self.ly,
            LCDC => self.lcdc,
            LYC => self.lyc,
            SCY => self.scy,
            SCX => self.scx,
            WY => self.wy,
            WX => self.wx,
            STAT => self.stat,
            _ => todo!("UNSUPPORTED READ 0x{:04X}", addr),
        }
    }

    pub fn write_byte(&mut self, addr: u16, b: u8) {
        match addr {
            0x8000..0xA000 => {
                if self.ppu_mode != Mode3 {
                    // println!("WRITING 0x{:02X} TO VRAM AT 0x{:04X}", b, addr);
                    self.write_vram(addr, b);
                } else {
                    println!("FAILED TO WRITE VRAM! MODE {:?}", self.ppu_mode);
                }
            }
            LCDC => self.lcdc = b,
            BGP => self.bgp = b,
            LY => (),
            LYC => self.lyc = b,
            SCY => self.scy = b,
            SCX => self.scx = b,
            WY => self.wy = b,
            WX => self.wx = b,
            STAT => self.stat = b,
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
            let tmap_base: u16 = 0x9800 | (bit(self.lcdc, 3) as u16) << 7;
            let tile_id: u8 = self.read_vram(tmap_base + tiley * 0x20 + tilex);
            let tile_addr: u16 = if bit(self.lcdc, 4) {
                0x8000 + 0x10 * tile_id as u16
            } else {
                todo!()
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
            self.display_buffer[self.ly as usize][lx as usize] = colour;
        }
    }

    // fn draw_bg(&mut self) {
    //     for px in 0..255u8 {
    //         let tmapx: u8 = self.scx.wrapping_add(px) % 32;
    //         let tmapy: u8 = self.scy.wrapping_add(self.ly) % 32;
    //         let tiley: u8 = py % 8;
    //         let tmap_addr: u16 = self.bg_tmap_area + (tmapy as u16 * 32) + tmapx as u16;
    //         // println!("READING TMAP 0x{:04X}", tmap_addr);
    //         let tile_addr: u16 = {
    //             if self.bg_win_tdata_area == 0x8000 {
    //                 // unsigned addressing
    //                 self.bg_win_tdata_area + (self.read_vram(tmap_addr) as u16 * 0x10)
    //             } else {
    //                 todo!("uh oh");
    //             }
    //         };
    //         // println!("READING TILE 0x{:04X}", tile_addr);
    //         let tile_lsb: u8 = self.read_vram(tile_addr + 2 * tiley as u16);
    //         let tile_msb: u8 = self.read_vram(tile_addr + 2 * tiley as u16 + 1);
    //         for i in 0..8u8 {
    //             let tilex = 8 - i;
    //             let colour_lsb = bit(tile_lsb, tilex);
    //             let colour_msb = bit(tile_msb, tilex);
    //             // println!("COLOUR MSB: {:} COLOUR LSB: {:}", colour_msb, colour_lsb);
    //             let colour: u8 = match (colour_msb, colour_lsb) {
    //                 (false, false) => self.bgp & 3,
    //                 (false, true) => (self.bgp >> 2) & 3,
    //                 (true, false) => (self.bgp >> 4) & 3,
    //                 (true, true) => (self.bgp >> 6) & 3,
    //             };
    //             self.display_buffer[py as usize][px as usize] = colour;
    //         }
    //     }
    // }

    fn read_vram(&self, addr: u16) -> u8 {
        self.vram[addr as usize - 0x8000]
    }

    fn write_vram(&mut self, addr: u16, b: u8) {
        self.vram[addr as usize - 0x8000] = b;
    }
}

struct FIFOPixel {
    pub colour: DMGColour,
}

enum DMGColour {
    Colour0,
    Colour1,
    Colour2,
    Colour3,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum PpuMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}
