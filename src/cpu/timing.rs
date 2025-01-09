#[derive(Default)]
pub struct Div {
    div: u16,
    div_wires: [bool; 4],
    pub tac: u8,
    pub tick: bool,
}

impl Div {
    pub fn new() -> Self {
        Self {
            div: 0xAB,
            div_wires: [false; 4],
            tac: 0,
            tick: false,
        }
    }

    pub fn boot(&mut self, div: u16, tac: u8) {
        self.div = div;
        self.tac = tac;
    }

    pub fn div(&self) -> u8 {
        (self.div >> 6) as u8
    }

    pub fn reset(&mut self) {
        self.div = 0;
        self.set_wires();
    }

    pub fn step(&mut self) {
        if self.div > 0x3FFF {
            self.div = 0;
        } else {
            self.div += 1;
        }
        self.set_wires();
    }

    fn set_wires(&mut self) {
        let div_wires: [bool; 4] = [
            bit(self.div, 7),
            bit(self.div, 1),
            bit(self.div, 3),
            bit(self.div, 5),
        ];
        let tac_freq: usize = self.tac as usize & 0b11;
        let tac_enable: bool = (self.tac >> 2) & 1 == 1;

        let falling_edge: bool = self.div_wires[tac_freq] && !div_wires[tac_freq];
        self.tick = falling_edge && tac_enable;
        self.div_wires = div_wires;
    }
}

#[derive(Default)]
pub struct Tima {
    pub tma: u8,
    tima: u8,
    write_tima: bool,
    load_tima: bool,
    ovf: bool,
    delay_in: bool,
    pub delay_out: bool,
}

impl Tima {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn cycle(&mut self, tick: bool) {
        if tick {
            (self.tima, self.ovf) = self.tima.overflowing_add(1);
        }
        self.set_wires();
        if self.load_tima {
            self.tima = self.tma;
        }
    }

    pub fn set_tima(&mut self, tima: u8) {
        self.tima = tima;
        self.write_tima = true;
    }

    pub fn tima(&self) -> u8 {
        self.tima
    }

    fn set_wires(&mut self) {
        self.delay_out = self.delay_in;
        self.load_tima = self.write_tima || self.delay_out;
        self.delay_in = !self.load_tima && self.ovf;
        self.write_tima = false;
    }
}

pub fn bit(b: u16, n: u8) -> bool {
    (b >> n) & 1 == 1
}
