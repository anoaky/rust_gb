use crate::constants::*;
use crate::ppu::DMGColour::{Colour0, Colour1, Colour2, Colour3};
use crate::ppu::PpuMode::*;
use bitvec::order::Msb0;
use bitvec::view::BitView;
use std::collections::VecDeque;

pub struct Ppu {
    vram: Vec<u8>,
    ppu_mode: PpuMode,
    pub ly: u8,
    bgp: u8,
    lx: u8,
    tmapx: u8,
    lcd_enable: bool,
    window_tmap_area: u16,
    window_enable: bool,
    bg_win_tdata_area: u16,
    bg_tmap_area: u16,
    obj_size: bool,
    obj_enable: bool,
    bg_win_pri: bool,
    display_buffer: Vec<Vec<u8>>,
    pub frame: Option<Vec<Vec<u8>>>,
    bg_fifo: VecDeque<FIFOPixel>,
    available_dots: u32,
    dots: u32,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: vec![0; 0x2000],
            ppu_mode: Mode2,
            ly: 0,
            bgp: 0,
            lx: 0,
            tmapx: 0,
            lcd_enable: false,
            window_tmap_area: 0x9800,
            window_enable: false,
            bg_win_tdata_area: 0x8800,
            bg_tmap_area: 0x9800,
            obj_size: false,
            obj_enable: false,
            bg_win_pri: false,
            display_buffer: vec![vec![0; 160]; 144],
            frame: None,
            bg_fifo: VecDeque::with_capacity(16),
            available_dots: 0,
            dots: 0,
        }
    }

    pub fn cycle(&mut self) {
        self.available_dots += 4;

        if self.ppu_mode == Mode0 {
            if self.available_dots + self.dots >= 456 {
                self.available_dots = (self.dots + self.available_dots) - 456;
                self.dots = 0;
                self.ly = (self.ly + 1) % 154;
                if self.ly == 144 {
                    self.ppu_mode = Mode1;
                    self.frame = Some(self.display_buffer.clone());
                } else if self.ly == 0 {
                    self.ppu_mode = Mode2;
                }
            }
        }

        if self.ppu_mode == Mode2 {
            if self.available_dots + self.dots >= 80 {
                self.available_dots = (self.dots + self.available_dots) - 80;
                self.dots = 80;
                self.ppu_mode = Mode3;
            } else {
                self.dots += self.available_dots;
                self.available_dots = 0;
            }
        }

        if self.ppu_mode == Mode3 {
            if self.dots < 92 {
                // 80 (Mode2) + 12
                if self.available_dots + self.dots >= 92 {
                    // ready to start drawing
                    self.available_dots = (self.dots + self.available_dots) - 92;
                    self.dots = 92;
                } else {
                    self.dots += self.available_dots;
                    self.available_dots = 0;
                }
            }
            while self.available_dots >= 8 {
                self.available_dots -= 8;
                self.dots += 8;
                self.fetch_bg();
                self.display_pixels();
            }
            if self.dots >= 172 {
                self.ppu_mode = Mode0;
            }
        }
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
            LY => self.ly,
            _ => todo!("UNSUPPORTED READ 0x{:04X}", addr),
        }
    }

    pub fn write_byte(&mut self, addr: u16, b: u8) {
        match addr {
            0x8000..0xA000 => {
                if self.ppu_mode != Mode3 {
                    self.write_vram(addr, b);
                }
            }
            LCDC => {
                self.lcd_enable = ((b >> 7) & 1) == 1;
                self.window_tmap_area = if ((b >> 6) & 1) == 1 { 0x9C00 } else { 0x9800 };
                self.window_enable = ((b >> 5) & 1) == 1;
                self.bg_win_tdata_area = if ((b >> 4) & 1) == 1 { 0x8000 } else { 0x8800 };
                self.bg_tmap_area = if ((b >> 3) & 1) == 1 { 0x9C00 } else { 0x9800 };
                self.obj_size = ((b >> 2) & 1) == 1;
                self.obj_enable = ((b >> 1) & 1) == 1;
                self.bg_win_pri = (b & 1) == 1;
            }
            BGP => {
                self.bgp = b;
            }
            LY => (),
            _ => todo!("UNSUPPORTED WRITE 0x{:04X}", addr),
        }
    }

    fn display_pixels(&mut self) {
        while let Some(pixel) = self.bg_fifo.pop_front() {
            let colour: u8 = match pixel.colour {
                Colour0 => self.bgp & 3,
                Colour1 => (self.bgp >> 2) & 3,
                Colour2 => (self.bgp >> 4) & 3,
                Colour3 => (self.bgp >> 6) & 3,
            };
            self.display_buffer[self.ly as usize][self.lx as usize] = colour;
            self.lx = (self.lx + 1) % 160;
        }
    }

    fn fetch_bg(&mut self) {
        let tmapy: u8 = self.ly / 8;
        let tiley: u8 = self.ly % 8;
        let tmap_addr: u16 = self.bg_tmap_area + tmapy as u16 * 20 + self.tmapx as u16;
        let tile_addr: u16 = {
            if self.bg_win_tdata_area == 0x8000 {
                // unsigned addressing
                self.bg_win_tdata_area + self.read_vram(tmap_addr) as u16
            } else {
                // signed addressing
                (0x9000 + self.read_vram(tmap_addr) as i8 as i16 as u16 as u32) as u16
            }
        };
        let tile_lo: u8 = self.read_vram(tile_addr + 2 * tiley as u16);
        let tile_hi: u8 = self.read_vram(tile_addr + 2 * tiley as u16 + 1);
        let lo_bits = tile_lo.view_bits::<Msb0>().to_bitvec();
        let hi_bits = tile_hi.view_bits::<Msb0>().to_bitvec();
        for tilex in 0..8usize {
            let colour: DMGColour = match (hi_bits[tilex], lo_bits[tilex]) {
                (false, false) => Colour0,
                (false, true) => Colour1,
                (true, false) => Colour2,
                (true, true) => Colour3,
            };
            self.bg_fifo.push_back(FIFOPixel { colour });
        }
        self.tmapx = (self.tmapx + 1) % 20; // 20 * 8 = 160
    }

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

#[derive(Eq, PartialEq)]
enum PpuMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}
