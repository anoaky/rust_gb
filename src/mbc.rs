use crate::cpu::combine_u8;
use std::fs::File;
use std::io::{BufReader, Read};
use std::pin::Pin;

pub fn make_mbc(fp: &str) -> Box<dyn Mbc + 'static> {
    let mut buf = Vec::new();
    BufReader::new(File::open(fp).unwrap())
        .read_to_end(&mut buf)
        .unwrap();
    match buf[0x0147] {
        0 => Box::new(Mbc0::new(buf)),
        _ => todo!("UNSUPPORTED MBC {:#04x}", buf[0x0147]),
    }
}

pub trait Mbc: Send {
    fn read_byte(&self, addr: u16) -> u8;
    fn read_word(&self, addr: u16) -> u16;
    fn boot(&self);
}

pub struct Mbc0 {
    data: Pin<Vec<u8>>,
}

impl Mbc for Mbc0 {
    fn read_byte(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }
    fn read_word(&self, addr: u16) -> u16 {
        combine_u8(self.data[addr as usize + 1], self.data[addr as usize])
    }
    fn boot(&self) {}
}

impl Mbc0 {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Pin::new(data),
        }
    }
}
