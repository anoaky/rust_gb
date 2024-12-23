use crate::mmu::Mmu;

const B: usize = 0;
const C: usize = 1;
const D: usize = 2;
const E: usize = 3;
const H: usize = 4;
const L: usize = 5;
const A: usize = 6;
const F: usize = 7;

pub struct Cpu {
    rg: Vec<u8>, // B, C, D, E, H, L, A, F
    io: Vec<u8>, // 0xFF00..0xFF80
    mmu: Mmu,
    sp: usize,
    stack: Vec<u8>,
    pc: u16,
    z: bool,
    n: bool,
    h: bool,
    c: bool,
}

impl Cpu {
    pub fn new(fp: &str) -> Self {
        Self {
            rg: vec![0; 8],
            io: vec![0; 0x80],
            mmu: Mmu::new(fp),
            sp: 0,
            stack: vec![0; u16::MAX as usize + 1],
            pc: 0x0100,
            z: false,
            n: false,
            h: false,
            c: false,
        }
    }

    pub fn exec(&mut self, opcode: u8) -> u32 {
        let (o2, o1, o0) = (
            (opcode & 0b1100_0000) >> 6,
            (opcode & 0b0011_1000) >> 3,
            (opcode & 0b0000_0111),
        );
        match (o2, o1, o0) {
            (0, 0, 0) => 1,                              // NOP
            (0, _, 2) => self.ld_ref(o2, o1),            // LD [r16 | A], [A | r16]
            (0, 3.., 0) => self.jr(o1),                  // JR (cc), e8
            (1, 6, 6) => unimplemented!("HALT"),         // HALT
            (1, _, _) => self.ld_8(o2, o1, o0),          // LD DST, SRC
            (2, _, _) => self.alu_r8(o1, o0),            // ALU r8
            (3, _, 6) => self.alu_n8(o1),                // ALU n8
            (3, 0..=3, 0) => self.ret_cc(o1),            // RET cc
            (3, 4 | 6, 0 | 2) => self.ldh(o1, o0),       // LDH
            (3, 5, 0) => unimplemented!("ADD SP, e8"),   // ADD SP, e8
            (3, 1, 1) => self.ret(),                     // RET
            (3, 3, 1) => unimplemented!("RETI"),         // RETI
            (3, 0 | 2 | 4 | 6, 1) => self.pop_r16(o1),   // POP r16
            (3, 0..=3, 2) => self.jp_cc(o1),             // JP cc, a16
            (3, 5 | 7, 2) => self.ld_ref(o2, o1),        // LD [a16 | A], [A | a16]
            (3, 0, 3) => self.jp_a16(),                  // JP a16
            (3, 0..=3, 4) => self.call_a16(self.cc(o1)), // CALL cc, a16
            (3, 1, 5) => self.call_a16(true),            // CALL a16
            (3, 0 | 2 | 4 | 6, 5) => self.push_r16(o1),  // PUSH r16
            (3, _, 7) => self.rst(o1),                   // RST vec
            _ => todo!("UNIMPLEMENTED {:#04x}", opcode),
        }
    }

    fn alu(&mut self, op: u8, src: u8) {
        match op {
            0 => {
                let a: u16 = self.rg[A] as u16 + src as u16;
                self.z = a == 0x100;
                self.n = false;
                self.h = self.rg[A] & 0xF + src & 0xF > 0xF;
                self.c = a > 0xFF;
                self.rg[A] = a as u8;
            } // ADD A, SRC
            1 => {
                let a: u16 = self.rg[A] as u16 + src as u16 + self.c as u16;
                self.z = a == 0x100;
                self.n = false;
                self.h = self.rg[A] & 0xF + src & 0xF + self.c as u8 & 0xF > 0xF;
                self.c = a > 0xFF;
                self.rg[A] = a as u8;
            } // ADC A, SRC
            2 => {
                let a = (self.rg[A] as u16) + (!src as u16 + 1);
                self.z = a == 0x100;
                self.n = true;
                self.h = self.rg[A] & 0xF < src & 0xF;
                self.c = a > 0x100;
                self.rg[A] = a as u8;
            } // SUB A, SRC
            3 => {
                let a: u16 = self.rg[A] as u16 + !(src as u16 + self.c as u16) + 1;
                self.z = a == 0x100;
                self.n = true;
                self.h = self.rg[A] & 0xF < (src & 0xF + self.c as u8);
                self.c = a > 0x100;
                self.rg[A] = a as u8;
            } // SBC A, SRC
            4 => {
                self.rg[A] &= src;
                self.z = self.rg[A] == 0;
                self.n = false;
                self.h = true;
                self.c = false;
            } // AND A, SRC
            5 => {
                self.rg[A] ^= src;
                self.z = self.rg[A] == 0;
                self.n = false;
                self.h = false;
                self.c = false;
            } // XOR A, SRC
            6 => {
                self.rg[A] |= src;
                self.z = self.rg[A] == 0;
                self.n = false;
                self.h = false;
                self.c = false;
            } // OR A, SRC
            7 => {
                let a: u16 = self.rg[A] as u16 + !src as u16 + 1;
                self.z = a == 0x100;
                self.n = true;
                self.h = self.rg[A] & 0xF < src & 0xF;
                self.c = a > 0x100;
            } // CP A, SRC
            _ => panic!("how"),
        }
    }

    fn alu_n8(&mut self, o1: u8) -> u32 {
        let src = self.next_byte();
        self.alu(o1, src);
        self.set_flags();
        2
    }

    fn alu_r8(&mut self, o1: u8, o0: u8) -> u32 {
        let src = self.r8_src(o0);
        self.alu(o1, src);
        self.set_flags();
        if o0 == 6 {
            2
        } else {
            1
        }
    }

    fn call(&mut self, addr: u16) {
        let (hi, lo): (u8, u8) = split_u16(self.pc);
        self.push(hi);
        self.push(lo);
        self.pc = addr;
    }

    fn call_a16(&mut self, cc: bool) -> u32 {
        let addr: u16 = self.next_word();
        if cc {
            self.call(addr);
            6
        } else {
            3
        }
    }

    fn cc(&self, o1: u8) -> bool {
        match o1 % 4 {
            0 => !self.z,
            1 => self.z,
            2 => !self.c,
            3 => self.c,
            _ => panic!("how"),
        }
    }

    fn jp_a16(&mut self) -> u32 {
        let addr: u16 = self.next_word();
        self.pc = addr;
        4
    }

    fn jp_cc(&mut self, o1: u8) -> u32 {
        let addr = self.next_word();
        if self.cc(o1) {
            self.pc = addr;
            4
        } else {
            3
        }
    }

    fn jr(&mut self, o1: u8) -> u32 {
        let off: u8 = self.next_byte();
        let cc: bool = {
            if o1 == 3 {
                true
            } else {
                self.cc(o1)
            }
        };
        if cc {
            self.pc = add_u16_e8(self.pc, off);
            3
        } else {
            2
        }
    }

    fn ld_8(&mut self, o2: u8, o1: u8, o0: u8) -> u32 {
        let cycles: u32 = 1 + (o1 == 6) as u32 + (o0 == 6) as u32;
        let src: u8 = {
            if o2 == 0 {
                self.next_byte()
            } else {
                self.r8_src(o0)
            }
        };
        match o1 {
            0 => self.rg[B] = src,
            1 => self.rg[C] = src,
            2 => self.rg[D] = src,
            3 => self.rg[E] = src,
            4 => self.rg[H] = src,
            5 => self.rg[L] = src,
            6 => self.mmu.write_byte(combine_u8(self.rg[H], self.rg[L]), src),
            7 => self.rg[A] = src,
            _ => todo!("how"),
        }
        cycles
    }

    fn ld_ref(&mut self, o2: u8, o1: u8) -> u32 {
        let cycles: u32 = if o2 == 0 { 2 } else { 4 };
        match (o2, o1) {
            (0, 0 | 2) => {
                let dest: u16 = combine_u8(self.rg[o1 as usize], self.rg[o1 as usize + 1]);
                self.write_byte(dest, self.rg[A]);
            } // LD [BC | DE], A
            (0, 1 | 3) => {
                let src: u16 = combine_u8(self.rg[o1 as usize - 1], self.rg[o1 as usize]);
                self.rg[A] = self.read_byte(src);
            } // LD A, [BC | DE]
            (0, 4 | 6) => {
                let addr: u16 = if o1 == 4 { self.hli() } else { self.hld() };
                self.write_byte(addr, self.rg[A]);
            } // LD [HLI | HLD], A
            (0, 5 | 7) => {
                let addr: u16 = if o1 == 5 { self.hli() } else { self.hld() };
                self.rg[A] = self.read_byte(addr);
            } // LD A, [HLI | HLD]
            (3, 5) => {
                let addr: u16 = self.next_word();
                self.write_byte(addr, self.rg[A]);
            } // LD [a16], A
            (3, 7) => {
                let addr: u16 = self.next_word();
                self.rg[A] = self.read_byte(addr);
            } // LD A, [a16]
            _ => (),
        }
        cycles
    }

    fn ldh(&mut self, o1: u8, o0: u8) -> u32 {
        let cycles: u32 = 2 + (o0 == 0) as u32;
        let addr: u16 = 0xFF00 + {
            if o0 == 0 {
                self.next_byte() as u16
            } else {
                self.rg[C] as u16
            }
        };
        if o1 == 4 {
            self.write_byte(addr, self.rg[A]);
        } else {
            self.rg[A] = self.read_byte(addr);
        }
        cycles
    }

    fn next_byte(&mut self) -> u8 {
        let byte = self.read_byte(self.pc);
        self.pc += 1;
        byte
    }

    fn next_word(&mut self) -> u16 {
        let word = self.mmu.read_word(self.pc);
        self.pc += 2;
        word
    }

    fn pop(&mut self) -> u8 {
        let b: u8 = self.stack[self.sp];
        self.sp = self.sp.wrapping_add(1);
        b
    }

    fn pop_r16(&mut self, o1: u8) -> u32 {
        match o1 {
            0 => {
                self.rg[C] = self.pop();
                self.rg[B] = self.pop();
            }
            2 => {
                self.rg[E] = self.pop();
                self.rg[D] = self.pop();
            }
            4 => {
                self.rg[L] = self.pop();
                self.rg[H] = self.pop();
            }
            6 => {
                self.rg[F] = self.pop();
                self.rg[A] = self.pop();
                self.read_flags();
            }
            _ => (),
        }
        3
    }

    fn push(&mut self, b: u8) {
        self.sp = self.sp.wrapping_sub(1);
        self.stack[self.sp] = b;
    }

    fn push_r16(&mut self, o1: u8) -> u32 {
        match o1 {
            0 => {
                self.push(self.rg[B]);
                self.push(self.rg[C]);
            }
            2 => {
                self.push(self.rg[D]);
                self.push(self.rg[E]);
            }
            4 => {
                self.push(self.rg[H]);
                self.push(self.rg[L]);
            }
            6 => {
                self.push(self.rg[A]);
                self.push(self.rg[F]);
            }
            _ => (),
        }
        4
    }

    fn r8_src(&self, o0: u8) -> u8 {
        match o0 {
            0 => self.rg[B],
            1 => self.rg[C],
            2 => self.rg[D],
            3 => self.rg[E],
            4 => self.rg[H],
            5 => self.rg[L],
            6 => self.read_byte(self.hl()),
            7 => self.rg[A],
            _ => todo!("how"),
        }
    }

    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0xFF00..0xFF80 => self.io[addr as usize - 0xFF00],
            _ => self.mmu.read_byte(addr),
        }
    }

    fn read_word(&mut self, addr: u16) -> u16 {
        match addr {
            _ => self.mmu.read_word(addr),
        }
    }

    fn read_flags(&mut self) {
        self.z = self.rg[F] >> 7 == 1;
        self.n = self.rg[F] >> 6 == 1;
        self.h = self.rg[F] >> 5 == 1;
        self.c = self.rg[F] >> 4 == 1;
    }

    fn ret(&mut self) -> u32 {
        let lo: u8 = self.pop();
        let hi: u8 = self.pop();
        self.pc = combine_u8(hi, lo);
        4
    }

    fn ret_cc(&mut self, o1: u8) -> u32 {
        if self.cc(o1) {
            self.ret() + 1
        } else {
            2
        }
    }

    fn rst(&mut self, o1: u8) -> u32 {
        let vec: u16 = o1 as u16 * 8;
        self.call(vec);
        4
    }

    fn set_flags(&mut self) {
        self.rg[F] =
            (self.z as u8) << 7 | (self.n as u8) << 6 | (self.h as u8) << 5 | (self.c as u8) << 4;
    }

    fn write_byte(&mut self, addr: u16, b: u8) {
        match addr {
            0xFF00..0xFF80 => self.io[addr as usize - 0xFF00] = b,
            _ => self.mmu.write_byte(addr, b),
        }
    }

    fn write_word(&mut self, addr: u16, w: u16) {
        match addr {
            _ => self.mmu.write_word(addr, w),
        }
    }

    // 16 BIT REGISTERS
    fn bc(&self) -> u16 {
        combine_u8(self.rg[B], self.rg[C])
    }

    fn de(&self) -> u16 {
        combine_u8(self.rg[D], self.rg[E])
    }

    fn hl(&self) -> u16 {
        combine_u8(self.rg[H], self.rg[L])
    }

    fn hld(&mut self) -> u16 {
        let hl: u16 = self.hl();
        (self.rg[H], self.rg[L]) = split_u16(hl.wrapping_sub(1));
        hl
    }

    fn hli(&mut self) -> u16 {
        let hl: u16 = self.hl();
        (self.rg[H], self.rg[L]) = split_u16(hl.wrapping_add(1));
        hl
    }
}

#[derive(Default)]
struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: Flags,
    pub h: u8,
    pub l: u8,
}

impl Registers {
    pub fn bc(&self) -> u16 {
        combine_u8(self.b, self.c)
    }
    pub fn de(&self) -> u16 {
        combine_u8(self.d, self.e)
    }
    pub fn hl(&self) -> u16 {
        combine_u8(self.h, self.l)
    }
}

#[derive(Default)]
struct Flags {
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
}

impl Into<u8> for Flags {
    fn into(self) -> u8 {
        (self.z as u8) << 7 | (self.n as u8) << 6 | (self.h as u8) << 5 | (self.c as u8) << 4
    }
}

pub fn add_u16_e8(a: u16, b: u8) -> u16 {
    (a as u32 + b as i8 as i16 as u16 as u32) as u16
}
pub fn combine_u8(hi: u8, lo: u8) -> u16 {
    (hi as u16) << 8 | lo as u16
}

pub fn split_u16(n: u16) -> (u8, u8) {
    let hi: u8 = (n >> 8) as u8;
    let lo: u8 = n as u8;
    (hi, lo)
}
