use crate::cpu::combine_u8;
use crate::mbc::Mbc;

pub struct Mbc0 {
    data: Vec<u8>,
}

impl Mbc for Mbc0 {
    fn boot(&self) {}

    fn read_byte(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    fn read_word(&self, addr: u16) -> u16 {
        combine_u8(self.data[addr as usize + 1], self.data[addr as usize])
    }
    fn write_byte(&mut self, addr: u16, b: u8) {
        ()
    }
}

impl Mbc0 {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}
