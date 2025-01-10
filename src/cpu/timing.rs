use crate::utils::*;
#[derive(Default)]
pub struct Timer {
    div: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    interrupt: bool,
    pub interrupt_requested: bool,
}

impl Timer {
    pub fn boot() -> Self {
        Self {
            div: 0xABCC,
            tac: 0xF8,
            ..Self::default()
        }
    }
    pub fn cycle(&mut self, cycles: u16) -> bool {
        self.interrupt_requested = false;

        let interrupt: bool = self.interrupt;
        if interrupt {
            self.tima = self.tma;
            self.interrupt_requested = true;
        }
        self.interrupt = false;
        let mut cycles = cycles;
        while cycles > 0 {
            let div: u16 = self.div;
            self.div = self.div.wrapping_add(1);
            self.tick(div);
            cycles -= 1;
        }

        interrupt
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        let r = match addr {
            DIV => (self.div >> 6) as u8,
            TIMA => self.tima,
            TMA => self.tma,
            TAC => self.tac,
            _ => unreachable!(),
        };
        return r;
    }

    pub fn write_byte(&mut self, addr: u16, b: u8) {
        match addr {
            DIV => {
                let div: u16 = self.div;
                self.div = 0;
                self.tick(div);
            }
            TIMA => {
                if !self.interrupt_requested {
                    self.tima = b;
                    self.interrupt = false;
                }
            }
            TMA => {
                self.tma = b;
                if self.interrupt_requested {
                    self.tima = b;
                }
            }
            TAC => self.tac = b,
            _ => unreachable!(),
        };
    }

    fn tick(&mut self, old: u16) {
        if bit(self.tac as u16, 2) && self.falling_edge(old) {
            (self.tima, self.interrupt) = self.tima.overflowing_add(1);
        }
    }

    fn falling_edge(&self, old: u16) -> bool {
        let shift: u8 = match self.tac & 3 {
            0 => 7,
            1 => 1,
            2 => 3,
            3 => 5,
            _ => unreachable!(),
        };
        return bit(old, shift) && !bit(self.div, shift);
    }
}
