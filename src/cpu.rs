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
        match opcode {
            0x00 => 1, // NOP
            0x01 => {
                (self.rg[B], self.rg[C]) = split_u16(self.next_word());
                3
            } // LD BC, n16
            0x02 => {
                self.write_byte(self.bc(), self.rg[A]);
                2
            } // LD [BC], A
            0x03 => {
                (self.rg[B], self.rg[C]) = split_u16(self.bc().wrapping_add(1));
                2
            } // INC BC
            0x04 => self.alu_inc(B), // INC B
            0x05 => self.alu_dec(B), // DEC B
            0x40..0x76 => self.ld_r8(opcode), // LD
            0x76 => todo!("HALT"), // HALT
            0x77..0x80 => self.ld_r8(opcode), // LD
            0x80..0xC0 => self.alu_r8(opcode), // ALU r8
            0xC0 => self.ret_cc(!self.z), // RET NZ
            0xC1 => self.pop_r16(B), // POP BC
            0xC2 => self.jp(!self.z), // JP NZ, a16
            0xC3 => self.jp(true), // JP a16
            0xC4 => self.call_a16(!self.z), // CALL NZ, a16
            0xC5 => self.push_r16(B), // PUSH BC
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => self.alu_n8(opcode), // ALU n8
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => self.rst(opcode), // RST $00
            0xC8 => self.ret_cc(self.z), // RET Z
            0xC9 => self.ret(), // RET
            0xCA => self.jp(self.z), // JP Z, a16
            0xCC => self.call_a16(self.z), // CALL Z, a16
            0xCD => self.call_a16(true), // CALL a16
            0xD0 => self.ret_cc(!self.c), // RET NC
            0xD1 => self.pop_r16(D), // POP DE
            0xD2 => self.jp(!self.c), // JP NC, a16
            0xD3 => panic!("ILLEGAL OPCODE"), // ILLEGAL
            0xD4 => self.call_a16(!self.c), // CALL NC, a16
            0xD5 => self.push_r16(D), // PUSH DE
            0xD8 => self.ret_cc(self.c), // RET C
            0xD9 => todo!("RETI"), // RETI
            0xDA => self.jp(self.c), // JP C, a16
            0xDB => panic!("ILLEGAL OPCODE"), // ILLEGAL
            0xDC => self.call_a16(self.c), // CALL C, a16
            0xDD => panic!("ILLEGAL OPCODE"), // ILLEGAL
            0xE1 => self.pop_r16(H), // POP HL
            0xE5 => self.push_r16(H), // PUSH HL
            0xF1 => self.pop_r16(A), // POP AF
            0xF5 => self.push_r16(A), // PUSH AF
            _ => todo!(),
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
            _ => (),
        }
    }

    fn alu_dec(&mut self, r: usize) -> u32 {
        let res: u8 = (self.rg[r] as u16 + 0xFF) as u8;
        self.z = res == 0;
        self.n = true;
        self.h = self.rg[r] & 0xF == 0;
        self.set_flags();
        1
    }

    fn alu_inc(&mut self, r: usize) -> u32 {
        let res: u8 = (self.rg[r] as u16 + 1) as u8;
        self.z = res == 0;
        self.n = false;
        self.h = self.rg[r] & 0xF == 0xF;
        self.rg[r] = res;
        self.set_flags();
        1
    }

    fn alu_n8(&mut self, opcode: u8) -> u32 {
        let op: u8 = (opcode - 0xC0) / 8;
        let src: u8 = self.next_byte();
        self.alu(op, src);
        2
    }

    fn alu_r8(&mut self, opcode: u8) -> u32 {
        let op: u8 = (opcode - 0x80) / 8;
        let src: u8 = self.r8_src(opcode);
        self.alu(op, src);
        1 + ((opcode & 0xF) % 8 == 6) as u32
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

    fn jp(&mut self, cc: bool) -> u32 {
        let addr = self.next_word();
        if cc {
            self.pc = addr;
            4
        } else {
            3
        }
    }

    fn ld_r8(&mut self, opcode: u8) -> u32 {
        let src: u8 = self.r8_src(opcode);
        let d: u8 = (opcode - 0x40) / 0x08;
        if d == 6 {
            self.write_byte(self.hl(), src);
        } else if d == 7 {
            self.rg[A] = src;
        } else {
            self.rg[d as usize] = src;
        }
        1 + (opcode & 0xF0 == 0x70) as u32 + ((opcode & 0x0F) % 8 == 6) as u32
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

    fn pop_r16(&mut self, r_hi: usize) -> u32 {
        self.rg[r_hi + 1] = self.pop();
        self.rg[r_hi] = self.pop();
        if r_hi == A {
            self.read_flags();
        }
        3
    }

    fn push(&mut self, b: u8) {
        self.sp = self.sp.wrapping_sub(1);
        self.stack[self.sp] = b;
    }

    fn push_r16(&mut self, r_hi: usize) -> u32 {
        self.push(self.rg[r_hi]);
        self.push(self.rg[r_hi + 1]);
        4
    }

    fn r8_src(&self, opcode: u8) -> u8 {
        let r: u8 = (opcode & 0x0F) % 0x08;
        if r == 6 {
            self.read_byte(self.hl())
        } else if r == 7 {
            self.rg[A]
        } else {
            self.rg[r as usize]
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

    fn ret_cc(&mut self, cc: bool) -> u32 {
        if cc {
            self.ret() + 1
        } else {
            2
        }
    }

    fn rst(&mut self, opcode: u8) -> u32 {
        let vec: u16 = (opcode % 8) as u16;
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
