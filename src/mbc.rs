use std::pin::Pin;

pub trait Mbc: Send {
    fn read(&self, addr: u16, buf: &mut [u8]);
    fn write(&mut self, addr: u16, buf: &mut [u8]);
    fn boot(&self);
}

pub struct Mbc0 {
    data: Pin<Vec<u8>>,
}

impl Mbc for Mbc0 {
    fn read(&self, addr: u16, buf: &mut [u8]) {
        let addr: u64 = addr as u64;
        let end: u64 = buf.len() as u64 + addr;
        match addr {
            0x0000..0x8000 => buf.copy_from_slice(&self.data[addr..end]),
            _ => panic!("MBC0 CANNOT READ {:#06x}", addr),
        }
    }
}

impl Mbc0 {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data: Pin::new(data),
        }
    }
}
