pub struct Cpu {
    rg: Registers,
    sp: u16,
    pc: u16,
}

#[derive(Default)]
struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
    pub h: u8,
    pub l: u8,
}