use crate::mbc::mbc0::Mbc0;
use crate::mbc::mbc1::Mbc1;
use std::fs::File;
use std::io::{BufReader, Read};

pub mod mbc0;
pub mod mbc1;

pub fn make_mbc(fp: &str) -> Box<dyn Mbc + 'static> {
    let mut buf = Vec::new();
    println!("{:}", fp);
    BufReader::new(File::open(fp).unwrap())
        .read_to_end(&mut buf)
        .unwrap();
    match buf[0x0147] {
        0 => Box::new(Mbc0::new(buf)),
        1 | 2 | 3 => Box::new(Mbc1::new(buf)),
        _ => todo!("UNSUPPORTED MBC {:#04x}", buf[0x0147]),
    }
}

pub trait Mbc: Send {
    fn boot(&self);
    fn read_byte(&self, addr: u16) -> u8;
    fn read_word(&self, addr: u16) -> u16;
    fn write_byte(&mut self, addr: u16, b: u8);
}

fn header_ram(v: u8) -> usize {
    match v {
        1 | 2 => 1,
        3 => 4,
        4 => 16,
        5 => 8,
        _ => 0,
    }
}

fn header_rom(v: u8) -> usize {
    2 << v
}
