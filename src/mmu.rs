use std::pin::Pin;

pub struct Mmu {
    wram_00: Pin<Vec<u8>>,
    wram_01: Pin<Vec<u8>>,
}