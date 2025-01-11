use std::ops::{BitOr, Shl};

pub const SB: u16 = 0xFF01;
pub const SC: u16 = 0xFF02;
pub const DIV: u16 = 0xFF04;
pub const TIMA: u16 = 0xFF05;
pub const TMA: u16 = 0xFF06;
pub const TAC: u16 = 0xFF07;
pub const LCDC: u16 = 0xFF40;
pub const STAT: u16 = 0xFF41;
pub const SCY: u16 = 0xFF42;
pub const SCX: u16 = 0xFF43;
pub const LY: u16 = 0xFF44;
pub const LYC: u16 = 0xFF45;
pub const DMA: u16 = 0xFF46;
pub const BGP: u16 = 0xFF47;
pub const OBP0: u16 = 0xFF48;
pub const OBP1: u16 = 0xFF49;
pub const WY: u16 = 0xFF4A;
pub const WX: u16 = 0xFF4B;
pub const IF: u16 = 0xFF0F;
pub const IE: u16 = 0xFFFF;

pub fn bit<T: Into<u32>>(b: T, bit: u8) -> bool {
    (b.into() >> bit) & 1 == 1
}

pub fn set<T: Into<u32>>(v: T, bit: u8) -> u32 {
    v.into() | (1u32 << bit)
}

pub fn unset<T: Into<u32>>(v: T, bit: u8) -> u32 {
    v.into() & !(1u32 << bit)
}

pub fn flag<T>(bit: u8) -> T::Output
where
    T: From<u8> + Shl,
    u8: Into<T>,
{
    1u8.into() << bit.into()
}
