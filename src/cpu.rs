use crate::constants::*;
use crate::cpu::R16::{AF, BC, DE, HL, SP};
use crate::cpu::R8::{A, B, C, D, E, F, H, HLA, L};
use crate::mmu::Mmu;
use anyhow::{bail, ensure, Result};

const DMG_REG: [u8; 8] = [0x00, 0x13, 0x00, 0xD8, 0x01, 0x4D, 0x01, 0x80];

pub struct Cpu {
    rg: Vec<u8>, // B, C, D, E, H, L, A, F
    io: Vec<u8>, // 0xFF00..0xFF80
    hram: Vec<u8>,
    pub sb: u8, // 0xFF01
    pub sc_enable: bool,
    sc_speed: bool,
    sc_select: bool,
    ime: bool,
    ei: bool,
    ienable: InterruptFlags,
    iflags: InterruptFlags,
    mmu: Mmu,
    sp: u16,
    pc: u16,
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
    div_acc: u32,
    tima_acc: u32,
    tac_enable: bool,
    tac_select: u32,
    halted: bool,
}

#[derive(Default, Copy, Clone)]
pub struct CpuTickOutput {
    pub m_cycles: u32,
    pub sb: Option<u8>,
}

impl Cpu {
    pub fn new(fp: &str) -> Self {
        Self {
            rg: vec![0; 8],
            io: vec![0; 0x80],
            hram: vec![0; 0x80],
            sb: 0,
            sc_enable: false,
            sc_speed: false,
            sc_select: false,
            ienable: InterruptFlags::default(),
            iflags: InterruptFlags::default(),
            ime: false,
            ei: false,
            mmu: Mmu::new(fp),
            sp: 0xFFFE,
            pc: 0x0100,
            z: false,
            n: false,
            h: false,
            c: false,
            div_acc: 0,
            tima_acc: 0,
            tac_enable: false,
            tac_select: 256,
            halted: false,
        }
    }

    pub fn boot(&mut self) {
        self.rg.copy_from_slice(&DMG_REG);
        self.write_byte(SB, 0);
        self.write_byte(SC, 0x7E);
        self.io[DIV as usize - 0xFF00] = 0x18;
        self.write_byte(TIMA, 0x00);
        self.write_byte(TMA, 0x00);
        self.write_byte(TAC, 0xF8);
        self.read_flags();
    }

    pub fn cycle(&mut self) -> u32 {
        if self.halted {
            1
        } else {
            let opcode: u8 = self.next_byte();
            let m_cycles: u32 = self.exec(opcode);
            if self.ei && opcode != 0xFB {
                self.ei = false;
                self.ime = true;
            }
            m_cycles
        }
    }

    pub fn tick(&mut self) -> CpuTickOutput {
        self.handle_interrupts();
        let mut to: CpuTickOutput = CpuTickOutput::default();
        let m_cycles: u32 = self.cycle();
        to.m_cycles = m_cycles;

        self.timers(m_cycles);
        if self.sc_enable {
            to.sb = Some(self.sb);
            self.sc_enable = false;
            self.iflags.serial = true;
        }
        to
    }

    pub fn exec(&mut self, opcode: u8) -> u32 {
        match opcode {
            0x00 => 1,                 // NOP
            0x01 => self.ld_imm16(BC), // LD BC, n16
            0x02 => {
                self.write_byte(self.bc(), self.rg[A as usize]);
                2
            } // LD [BC], A
            0x03 => self.alu_inc16(BC), // INC BC
            0x04 => self.alu_inc8(B),  // INC B
            0x05 => self.alu_dec8(B),  // DEC B
            0x06 => self.ld_imm8(B),   // LD B, n8
            0x07 => self.rlca(),       // RLCA
            0x08 => {
                let addr: u16 = self.next_word();
                self.write_word(addr, self.sp as u16);
                5
            } // LD [a16], SP
            0x09 => self.alu_add16(BC), // ADD HL, BC
            0x0A => {
                self.rg[A as usize] = self.read_byte(self.bc());
                2
            } // LD A, [BC]
            0x0B => self.alu_dec16(BC), // DEC BC
            0x0C => self.alu_inc8(C),  // INC C
            0x0D => self.alu_dec8(C),  // DEC C
            0x0E => self.ld_imm8(C),   // LD C, n8
            0x0F => self.rrca(),       // RRCA
            0x10 => self.stop(),       // STOP
            0x11 => self.ld_imm16(DE), // LD DE, n16
            0x12 => {
                self.write_byte(self.de(), self.rg[A as usize]);
                2
            } // LD [DE], A
            0x13 => self.alu_inc16(DE), // INC DE
            0x14 => self.alu_inc8(D),  // INC D
            0x15 => self.alu_dec8(D),  // DEC D
            0x16 => self.ld_imm8(D),   // LD D, n8
            0x17 => self.rla(),        // RLA
            0x18 => self.jr(true),     // JR e8
            0x19 => self.alu_add16(DE), // ADD HL, DE
            0x1A => {
                self.rg[A as usize] = self.read_byte(self.de());
                2
            } // LD A, [DE]
            0x1B => self.alu_dec16(DE), // DEC DE
            0x1C => self.alu_inc8(E),  // INC E
            0x1D => self.alu_dec8(E),  // DEC E
            0x1E => self.ld_imm8(E),   // LD E, n8
            0x1F => self.rra(),        // RRA
            0x20 => self.jr(!self.z),  // JR NZ, e8
            0x21 => self.ld_imm16(HL), // LD HL, n16
            0x22 => {
                let hl: u16 = self.hli();
                self.write_byte(hl, self.rg[A as usize]);
                2
            } // LD [HLI], A
            0x23 => self.alu_inc16(HL), // INC HL
            0x24 => self.alu_inc8(H),  // INC H
            0x25 => self.alu_dec8(H),  // DEC H
            0x26 => self.ld_imm8(H),   // LD H, n8
            0x27 => self.alu_daa(),    // DAA
            0x28 => self.jr(self.z),   // JR Z, e8
            0x29 => self.alu_add16(HL), // ADD HL, HL
            0x2A => {
                let hl: u16 = self.hli();
                self.rg[A as usize] = self.read_byte(hl);
                2
            } // LD A, [HLI]
            0x2B => self.alu_dec16(HL), // DEC HL
            0x2C => self.alu_inc8(L),  // INC L
            0x2D => self.alu_dec8(L),  // DEC L
            0x2E => self.ld_imm8(L),   // LD L, n8
            0x2F => self.alu_cpl(),    // CPL
            0x30 => self.jr(!self.c),  // JR NC, e8
            0x31 => {
                self.sp = self.next_word();
                3
            } // LD SP, n16
            0x32 => {
                let hld: u16 = self.hld();
                self.write_byte(hld, self.rg[A as usize]);
                2
            } // LD [HLD], A
            0x33 => {
                self.sp = self.sp.wrapping_add(1);
                2
            } // INC SP
            0x34 => self.alu_inc8(HLA), // INC [HL]
            0x35 => self.alu_dec8(HLA), // DEC [HL]
            0x36 => self.ld_imm8(HLA), // LD [HL], n8
            0x37 => {
                self.n = false;
                self.h = false;
                self.c = true;
                self.set_flags();
                1
            } // SCF
            0x38 => self.jr(self.c),   // JR C, e8
            0x39 => self.alu_add16(SP), // ADD HL, SP
            0x3A => {
                let hld: u16 = self.hld();
                self.rg[A as usize] = self.read_byte(hld);
                2
            } // LD A, [HLD]
            0x3B => self.alu_dec16(SP), // DEC SP
            0x3C => self.alu_inc8(A),  // INC A
            0x3D => self.alu_dec8(A),  // DEC A
            0x3E => self.ld_imm8(A),   // LD A, n8
            0x3F => {
                self.n = false;
                self.h = false;
                self.c = !self.c;
                self.set_flags();
                1
            } // CCF
            0x40..0x76 => self.ld_r8(opcode), // LD
            0x76 => self.halt(),       // HALT
            0x77..0x80 => self.ld_r8(opcode), // LD
            0x80..0xC0 => self.alu_r8(opcode), // ALU r8
            0xC0 => self.ret_cc(!self.z), // RET NZ
            0xC1 => self.pop_r16(BC),  // POP BC
            0xC2 => self.jp(!self.z),  // JP NZ, a16
            0xC3 => self.jp(true),     // JP a16
            0xC4 => self.call_a16(!self.z), // CALL NZ, a16
            0xC5 => self.push_r16(BC), // PUSH BC
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => self.alu_n8(opcode), // ALU n8
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => self.rst(opcode), // RST $00
            0xC8 => self.ret_cc(self.z), // RET Z
            0xC9 => self.ret(),        // RET
            0xCA => self.jp(self.z),   // JP Z, a16
            0xCB => self.cb(),         // PREFIX
            0xCC => self.call_a16(self.z), // CALL Z, a16
            0xCD => self.call_a16(true), // CALL a16
            0xD0 => self.ret_cc(!self.c), // RET NC
            0xD1 => self.pop_r16(DE),  // POP DE
            0xD2 => self.jp(!self.c),  // JP NC, a16
            0xD3 => panic!("ILLEGAL D3"), // ILLEGAL D3
            0xD4 => self.call_a16(!self.c), // CALL NC, a16
            0xD5 => self.push_r16(DE), // PUSH DE
            0xD8 => self.ret_cc(self.c), // RET C
            0xD9 => self.reti(),       // RETI
            0xDA => self.jp(self.c),   // JP C, a16
            0xDB => self.debug_print(), // ILLEGAL DB TODO
            0xDC => self.call_a16(self.c), // CALL C, a16
            0xDD => panic!("ILLEGAL DD"), // ILLEGAL DD
            0xE0 | 0xE2 | 0xF0 | 0xF2 => self.ldh(opcode), // LDH
            0xE1 => self.pop_r16(HL),  // POP HL
            0xE3 => panic!("ILLEGAL E3"), // ILLEGAL E3
            0xE4 => panic!("ILLEGAL E4"), // ILLEGAL E4
            0xE5 => self.push_r16(HL), // PUSH HL
            0xE8 => {
                let e8: u8 = self.next_byte();
                let (res, _, h, c) = add_u16_e8(self.sp as u16, e8);
                self.z = false;
                self.n = false;
                self.h = h;
                self.c = c;
                self.sp = res;
                self.set_flags();
                4
            } // ADD SP, e8
            0xE9 => {
                self.pc = self.hl();
                1
            } // JP HL
            0xEA => {
                let addr: u16 = self.next_word();
                self.write_byte(addr, self.rg[A as usize]);
                4
            } // LD [a16], A
            0xEB => panic!("ILLEGAL EB"), // ILLEGAL EB
            0xEC => panic!("ILLEGAL EC"), // ILLEGAL EC
            0xED => panic!("ILLEGAL ED"), // ILLEGAL ED
            0xF1 => self.pop_r16(AF),  // POP AF
            0xF3 => {
                self.ei = false;
                self.ime = false;
                1
            } // DI
            0xF4 => panic!("ILLEGAL F4"), // ILLEGAL F4
            0xF5 => self.push_r16(AF), // PUSH AF
            0xF8 => {
                let e8: u8 = self.next_byte();
                let (sp, _, h, c) = add_u16_e8(self.sp as u16, e8);
                self.z = false;
                self.n = false;
                self.h = h;
                self.c = c;
                (self.rg[H as usize], self.rg[L as usize]) = split_u16(sp);
                self.set_flags();
                3
            } // LD HL, SP + e8
            0xF9 => {
                self.sp = self.hl();
                2
            } // LD SP, HL
            0xFA => {
                let addr: u16 = self.next_word();
                self.rg[A as usize] = self.read_byte(addr);
                4
            } // LD A, [a16]
            0xFB => {
                self.ei = true;
                1
            } // EI
            0xFC => panic!("ILLEGAL FC"), // ILLEGAL FC
            0xFD => panic!("ILLEGAL FD"), // ILLEGAL FD
        }
    }

    pub fn cb(&mut self) -> u32 {
        let opcode: u8 = self.next_byte();
        let r: R8 = R8::try_from((opcode & 0x0F) % 8).unwrap();
        let src: u8 = {
            if r == HLA {
                self.read_byte(self.hl())
            } else {
                self.rg[r as usize]
            }
        };
        let res: u8 = match opcode {
            0x00..0x08 => self.rlc(src),
            0x08..0x10 => self.rrc(src),
            0x10..0x18 => self.rl(src),
            0x18..0x20 => self.rr(src),
            0x20..0x28 => self.sla(src),
            0x28..0x30 => self.sra(src),
            0x30..0x38 => self.swap(src),
            0x38..0x40 => self.srl(src),
            0x40..0x80 => self.alu_bit(src, (opcode - 0x40) / 8),
            0x80..0xC0 => self.alu_res(src, (opcode - 0x80) / 8),
            0xC0.. => self.alu_set(src, (opcode - 0xC0) / 8),
        };
        if r == HLA {
            if opcode < 0x40 || opcode >= 0x80 {
                self.write_byte(self.hl(), res);
            }
            3 + (opcode < 0x40 || opcode >= 0x80) as u32
        } else {
            if opcode < 0x40 || opcode >= 0x80 {
                self.rg[r as usize] = res;
            }
            2
        }
    }

    fn alu(&mut self, op: u8, src: u8) {
        match op {
            0 => self.alu_add(src, false), // ADD A, SRC
            1 => self.alu_add(src, true),  // ADC A, SRC
            2 => self.alu_sub(src, false), // SUB A, SRC
            3 => self.alu_sub(src, true),  // SBC A, SRC
            4 => {
                self.rg[A as usize] &= src;
                self.z = self.rg[A as usize] == 0;
                self.n = false;
                self.h = true;
                self.c = false;
            } // AND A, SRC
            5 => {
                self.rg[A as usize] ^= src;
                self.z = self.rg[A as usize] == 0;
                self.n = false;
                self.h = false;
                self.c = false;
            } // XOR A, SRC
            6 => {
                self.rg[A as usize] |= src;
                self.z = self.rg[A as usize] == 0;
                self.n = false;
                self.h = false;
                self.c = false;
            } // OR A, SRC
            7 => {
                let a: u8 = self.rg[A as usize];
                self.alu_sub(src, false);
                // if (src == self.rg[E as usize]) && (self.read_byte(self.pc) == 0xC2) {
                //     println!("CP {:#04x} {:#04x}", a, src);
                //     if a != src {
                //         println!("STACK: {:#04x?}", &self.stack[self.sp - 2..self.sp + 2]);
                //         println!("REGS: {:#04x?}", self.rg);
                //         panic!()
                //     }
                // }
                self.rg[A as usize] = a;
            } // CP A, SRC
            _ => (),
        }
        self.set_flags();
    }

    fn alu_add(&mut self, b: u8, carry: bool) {
        let a: u8 = self.rg[A as usize];
        let c: u8 = if carry && self.c { 1 } else { 0 };
        let res: u8 = a.wrapping_add(b).wrapping_add(c);
        self.z = res == 0;
        self.n = false;
        self.h = (a & 0x0F) + (b & 0x0F) + c > 0x0F;
        self.c = (a as u16) + (b as u16) + (c as u16) > 0xFF;
        self.rg[A as usize] = res;
    }

    fn alu_add16(&mut self, r: R16) -> u32 {
        let a: u16 = self.hl();
        let b: u16 = match r {
            BC => self.bc(),
            DE => self.de(),
            HL => self.hl(),
            AF => self.af(),
            SP => self.sp as u16,
        };
        let res: u32 = a as u32 + b as u32;
        self.n = false;
        self.h = (a & 0x0FFF) + (b & 0x0FFF) > 0x0FFF;
        self.c = res > 0xFFFF;
        self.set_flags();
        (self.rg[H as usize], self.rg[L as usize]) = split_u16(res as u16);
        2
    }

    fn alu_bit(&mut self, src: u8, shift: u8) -> u8 {
        if (src >> shift) & 1 == 1 {
            self.z = false;
        } else {
            self.z = true;
        }
        self.n = false;
        self.h = true;
        self.set_flags();
        0
    }

    fn alu_cpl(&mut self) -> u32 {
        let a: u8 = self.rg[A as usize];
        self.rg[A as usize] = !a;
        self.n = true;
        self.h = true;
        self.set_flags();
        1
    }

    fn alu_daa(&mut self) -> u32 {
        let mut a: u8 = self.rg[A as usize];
        if !self.n {
            if self.c || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.c = true;
            }
            if self.h || (a & 0x0F) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if self.c {
                a = a.wrapping_sub(0x60);
            }
            if self.h {
                a = a.wrapping_sub(0x06);
            }
        }
        self.z = a == 0;
        self.h = false;
        self.rg[A as usize] = a;
        self.set_flags();
        1
    }

    fn alu_dec8(&mut self, r: R8) -> u32 {
        let b: u8 = {
            if r == HLA {
                self.read_byte(self.hl())
            } else {
                self.rg[r as usize]
            }
        };
        let res: u8 = b.wrapping_sub(1);
        self.z = res == 0;
        self.n = true;
        self.h = b & 0xF == 0;
        self.set_flags();
        if r == HLA {
            self.write_byte(self.hl(), res);
            3
        } else {
            self.rg[r as usize] = res;
            1
        }
    }

    fn alu_dec16(&mut self, r: R16) -> u32 {
        let b: u16 = match r {
            BC => self.bc(),
            DE => self.de(),
            HL => self.hl(),
            AF => panic!(),
            SP => self.sp as u16,
        };
        let res: u16 = b.wrapping_sub(1);
        if r == SP {
            self.sp = res;
        } else {
            let (rh, rl): (R8, R8) = r16_to_hi_lo(r);
            (self.rg[rh as usize], self.rg[rl as usize]) = split_u16(res);
        }
        self.set_flags();
        2
    }

    fn alu_inc8(&mut self, r: R8) -> u32 {
        let b: u8 = {
            if r == HLA {
                self.read_byte(self.hl())
            } else {
                self.rg[r as usize]
            }
        };
        let res: u8 = b.wrapping_add(1);
        self.z = res == 0;
        self.n = false;
        self.h = b & 0x0F == 0x0F;
        self.set_flags();
        if r == HLA {
            self.write_byte(self.hl(), res);
            3
        } else {
            self.rg[r as usize] = res;
            1
        }
    }

    fn alu_inc16(&mut self, r: R16) -> u32 {
        let b: u16 = match r {
            BC => self.bc(),
            DE => self.de(),
            HL => self.hl(),
            AF => panic!(),
            SP => self.sp as u16,
        };
        let res: u16 = b.wrapping_add(1);
        if r == SP {
            self.sp = res;
        } else {
            let (rh, rl): (R8, R8) = r16_to_hi_lo(r);
            (self.rg[rh as usize], self.rg[rl as usize]) = split_u16(res);
        }
        self.set_flags();
        2
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

    fn alu_res(&mut self, src: u8, shift: u8) -> u8 {
        let mask: u8 = !(1 << shift);
        src & mask
    }

    fn alu_set(&mut self, src: u8, shift: u8) -> u8 {
        let mask: u8 = 1 << shift;
        src | mask
    }

    fn alu_sub(&mut self, b: u8, carry: bool) {
        let a: u8 = self.rg[A as usize];
        let c: u8 = if carry && self.c { 1 } else { 0 };
        let res: u8 = a.wrapping_sub(b).wrapping_sub(c);
        self.z = res == 0;
        self.n = true;
        self.h = (a & 0x0F) < (b & 0x0F) + c;
        self.c = (a as u16) < (b as u16) + (c as u16);
        self.rg[A as usize] = res;
    }

    fn call(&mut self, addr: u16) {
        self.push(self.pc);
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

    fn halt(&mut self) -> u32 {
        if self.ime || self.ienable.read() & self.iflags.read() == 0 {
            self.halted = true;
            0
        } else {
            // HALT BUG
            let opcode: u8 = self.next_byte();
            self.pc -= 1;
            self.exec(opcode)
        }
    }

    fn handle_interrupts(&mut self) {
        let pending: bool = self.ienable.read() & self.iflags.read() != 0;
        let mut handler: u16 = 0;
        if self.ime {
            if self.ienable.vblank && self.iflags.vblank {
                self.iflags.vblank = false;
                handler = 0x40;
            } else if self.ienable.lcd && self.iflags.lcd {
                self.iflags.lcd = false;
                handler = 0x48;
            } else if self.ienable.timer && self.iflags.timer {
                self.iflags.timer = false;
                handler = 0x50;
            } else if self.ienable.serial && self.iflags.serial {
                self.iflags.serial = false;
                handler = 0x58;
            } else if self.ienable.joy && self.iflags.joy {
                self.iflags.joy = false;
                handler = 0x60;
            }
        }

        if self.ime && pending {
            self.ime = false;
            self.call(handler);
            self.halted = false;
        } else if self.halted && pending {
            self.halted = false;
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

    fn jr(&mut self, cc: bool) -> u32 {
        let e8: u8 = self.next_byte();
        let (addr, _, _, _) = add_u16_e8(self.pc, e8);
        if cc {
            self.pc = addr;
            3
        } else {
            2
        }
    }

    fn ld_imm8(&mut self, r: R8) -> u32 {
        let b: u8 = self.next_byte();
        if r == HLA {
            self.write_byte(self.hl(), b);
        } else {
            self.rg[r as usize] = b;
        }
        2 + (r == HLA) as u32
    }

    fn ld_imm16(&mut self, r: R16) -> u32 {
        let w: u16 = self.next_word();
        let (rh, rl): (R8, R8) = match r {
            BC => (B, C),
            DE => (D, E),
            HL => (H, L),
            _ => panic!(),
        };
        (self.rg[rh as usize], self.rg[rl as usize]) = split_u16(w);
        3
    }

    fn ld_r8(&mut self, opcode: u8) -> u32 {
        let src: u8 = self.r8_src(opcode);
        let d: u8 = (opcode - 0x40) / 0x08;
        if d == 6 {
            self.write_byte(self.hl(), src);
        } else if d == 7 {
            self.rg[A as usize] = src;
        } else {
            self.rg[d as usize] = src;
        }
        1 + (opcode & 0xF0 == 0x70) as u32 + ((opcode & 0x0F) % 8 == 6) as u32
    }

    fn ldh(&mut self, opcode: u8) -> u32 {
        let c: bool = opcode & 0x0F == 0x02;
        let rd: bool = opcode & 0xF0 == 0xF0;
        let addr: u16 = {
            if c {
                0xFF00 + self.rg[C as usize] as u16
            } else {
                0xFF00 + self.next_byte() as u16
            }
        };
        if rd {
            self.rg[A as usize] = self.read_byte(addr);
        } else {
            self.write_byte(addr, self.rg[A as usize]);
        }
        2 + c as u32
    }

    fn next_byte(&mut self) -> u8 {
        let byte = self.read_byte(self.pc);
        self.pc += 1;
        byte
    }

    fn next_word(&mut self) -> u16 {
        let word = self.read_word(self.pc);
        self.pc += 2;
        word
    }

    fn pop(&mut self) -> u16 {
        let r: u16 = self.read_word(self.sp);
        self.sp += 2;
        r
    }

    fn pop_r16(&mut self, r: R16) -> u32 {
        let (r_hi, r_lo) = r16_to_hi_lo(r);
        (self.rg[r_hi as usize], self.rg[r_lo as usize]) = split_u16(self.pop());
        if r == AF {
            self.read_flags();
        }
        3
    }

    fn push(&mut self, b: u16) {
        self.write_word(self.sp - 2, b);
        self.sp -= 2;
        // println!("PUSH {:#06x}", b);
    }

    fn push_r16(&mut self, r: R16) -> u32 {
        match r {
            BC => self.push(self.bc()),
            DE => self.push(self.de()),
            HL => self.push(self.hl()),
            AF => {
                let af: u16 = self.af();
                self.push(af);
            }
            SP => panic!(),
        }
        4
    }

    fn r8_src(&self, opcode: u8) -> u8 {
        let r: u8 = (opcode & 0x0F) % 0x08;
        if r == 6 {
            self.read_byte(self.hl())
        } else if r == 7 {
            self.rg[A as usize]
        } else {
            self.rg[r as usize]
        }
    }

    fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            SB => self.sb,
            SC => self.read_sc(),
            IF => self.iflags.read(),
            0xFF00..0xFF80 => self.io[addr as usize - 0xFF00],
            0xFF80..0xFFFF => self.hram[addr as usize - 0xFF80],
            IE => self.ienable.read(),
            _ => self.mmu.read_byte(addr),
        }
    }

    fn read_flags(&mut self) {
        let f: u8 = self.rg[F as usize];
        // println!("READ FLAGS: {:#010b}", f);
        self.z = (f & 0x80) >> 7 == 1;
        self.n = (f & 0x40) >> 6 == 1;
        self.h = (f & 0x20) >> 5 == 1;
        self.c = (f & 0x10) >> 4 == 1;
        // println!(
        //     "Z: {:} N: {:} H: {:} C: {:}",
        //     self.z, self.n, self.h, self.c
        // );
    }

    fn read_sc(&self) -> u8 {
        (self.sc_enable as u8) << 7 | (self.sc_speed as u8) << 1 | self.sc_select as u8
    }

    fn read_word(&self, addr: u16) -> u16 {
        let lo: u8 = self.read_byte(addr);
        let hi: u8 = self.read_byte(addr + 1);
        combine_u8(hi, lo)
    }

    fn ret(&mut self) -> u32 {
        self.pc = self.pop();
        4
    }

    fn reti(&mut self) -> u32 {
        self.ime = true;
        self.ret()
    }

    fn ret_cc(&mut self, cc: bool) -> u32 {
        if cc {
            self.ret() + 1
        } else {
            2
        }
    }

    fn rl(&mut self, b: u8) -> u8 {
        let c: bool = b & 0x80 == 0x80;
        let res: u8 = (b << 1) | if self.c { 1 } else { 0 };
        self.sh_flags(res, c);
        res
    }

    fn rla(&mut self) -> u32 {
        self.rg[A as usize] = self.rl(self.rg[A as usize]);
        if self.z {
            self.z = false;
            self.set_flags();
        }
        1
    }

    fn rlc(&mut self, b: u8) -> u8 {
        let c: bool = b & 0x80 == 0x80;
        let res: u8 = b.rotate_left(1);
        self.sh_flags(res, c);
        res
    }

    fn rlca(&mut self) -> u32 {
        self.rg[A as usize] = self.rlc(self.rg[A as usize]);
        if self.z {
            self.z = false;
            self.set_flags();
        }
        1
    }

    fn rr(&mut self, b: u8) -> u8 {
        let c: bool = b & 1 == 1;
        let res: u8 = (b >> 1) | if self.c { 0x80 } else { 0 };
        self.sh_flags(res, c);
        res
    }

    fn rra(&mut self) -> u32 {
        self.rg[A as usize] = self.rr(self.rg[A as usize]);
        if self.z {
            self.z = false;
            self.set_flags();
        }
        1
    }

    fn rrc(&mut self, b: u8) -> u8 {
        let c: bool = b & 1 == 1;
        let res: u8 = (b >> 1) | if c { 0x80 } else { 0 };
        self.sh_flags(res, c);
        res
    }

    fn rrca(&mut self) -> u32 {
        self.rg[A as usize] = self.rrc(self.rg[A as usize]);
        if self.z {
            self.z = false;
            self.set_flags();
        }
        1
    }

    fn rst(&mut self, opcode: u8) -> u32 {
        let vec: u16 = (opcode - 0xC7) as u16;
        self.call(vec);
        4
    }

    fn serial_control(&mut self, sc: u8) {
        self.sc_enable = ((sc & 0x80) >> 7) == 1;
        self.sc_speed = ((sc & 0x02) >> 1) == 1;
        self.sc_select = sc & 1 == 1;
    }

    fn set_flags(&mut self) {
        self.rg[F as usize] =
            (self.z as u8) << 7 | (self.n as u8) << 6 | (self.h as u8) << 5 | (self.c as u8) << 4;
    }

    fn sh_flags(&mut self, res: u8, c: bool) {
        self.z = res == 0;
        self.n = false;
        self.h = false;
        self.c = c;
        self.set_flags();
    }

    fn sla(&mut self, b: u8) -> u8 {
        let c: bool = b & 0x80 == 0x80;
        let res: u8 = b << 1;
        self.sh_flags(res, c);
        res
    }

    fn sra(&mut self, b: u8) -> u8 {
        let c: bool = b & 1 == 1;
        let res: u8 = (b >> 1) | (b & 0x80);
        self.sh_flags(res, c);
        res
    }

    fn srl(&mut self, b: u8) -> u8 {
        let c: bool = b & 1 == 1;
        let res: u8 = b >> 1;
        self.sh_flags(res, c);
        res
    }

    fn stop(&mut self) -> u32 {
        self.next_byte();
        1
    }

    fn swap(&mut self, b: u8) -> u8 {
        self.sh_flags(b, false);
        (b << 4) | (b >> 4)
    }

    fn timers(&mut self, m_cycles: u32) {
        self.div_acc += m_cycles;
        if self.tac_enable {
            self.tima_acc += m_cycles;
        }

        while self.div_acc >= 64 {
            self.div_acc -= 64;
            let div: u8 = self.read_byte(DIV);
            self.write_byte(DIV, div.wrapping_add(1));
        }

        while self.tac_enable && self.tima_acc >= self.tac_select {
            self.tima_acc -= self.tac_select;
            let tima: u8 = self.read_byte(TIMA);
            if tima == 0xFF {
                let tma: u8 = self.read_byte(TMA);
                self.write_byte(TIMA, tma);
                self.iflags.timer = true;
            } else {
                self.write_byte(TIMA, tima + 1);
            }
        }
    }

    fn write_byte(&mut self, addr: u16, b: u8) {
        match addr {
            SB => self.sb = b,
            SC => self.serial_control(b),
            IF => self.iflags.set(b),
            DIV => self.io[DIV as usize - 0xFF00] = 0,
            TAC => {
                self.tac_select = {
                    let bits: u8 = b & 0b11;
                    if bits == 0b00 {
                        256
                    } else if bits == 0b01 {
                        4
                    } else if bits == 0b10 {
                        16
                    } else {
                        64
                    }
                };
                self.tac_enable = (b >> 2) & 1 == 1;
                self.io[TAC as usize - 0xFF00] = b;
            }
            0xFF00..0xFF80 => self.io[addr as usize - 0xFF00] = b,
            0xFF80..0xFFFF => self.hram[addr as usize - 0xFF80] = b,
            IE => self.ienable.set(b),
            _ => self.mmu.write_byte(addr, b),
        }
    }

    fn write_word(&mut self, addr: u16, w: u16) {
        let (hi, lo) = split_u16(w);
        self.write_byte(addr, lo);
        self.write_byte(addr + 1, hi);
    }

    // 16 BIT REGISTERS
    fn af(&mut self) -> u16 {
        self.set_flags();
        combine_u8(self.rg[A as usize], self.rg[F as usize])
    }

    fn bc(&self) -> u16 {
        combine_u8(self.rg[B as usize], self.rg[C as usize])
    }

    fn de(&self) -> u16 {
        combine_u8(self.rg[D as usize], self.rg[E as usize])
    }

    fn hl(&self) -> u16 {
        combine_u8(self.rg[H as usize], self.rg[L as usize])
    }

    fn hld(&mut self) -> u16 {
        let hl: u16 = self.hl();
        (self.rg[H as usize], self.rg[L as usize]) = split_u16(hl.wrapping_sub(1));
        hl
    }

    fn hli(&mut self) -> u16 {
        let hl: u16 = self.hl();
        (self.rg[H as usize], self.rg[L as usize]) = split_u16(hl.wrapping_add(1));
        hl
    }

    fn debug_print(&self) -> u32 {
        println!("RG: {:#04x?}", self.rg);
        println!("SP: {:#06x}", self.sp);
        println!("STACK: {:#06x}", self.read_word(self.sp));
        0
    }
}

pub fn add_u16_e8(a: u16, b: u8) -> (u16, bool, bool, bool) {
    // yes, these casts should be in this order
    let b: u16 = b as i8 as i16 as u16;
    let res: u32 = a as u32 + b as u32;
    let z: bool = res as u16 == 0;
    let h: bool = (a & 0x0F) + (b & 0x0F) > 0x0F;
    let c: bool = (a & 0xFF) + (b & 0xFF) > 0xFF;
    (res as u16, z, h, c)
}
pub fn combine_u8(hi: u8, lo: u8) -> u16 {
    (hi as u16) << 8 | lo as u16
}

pub fn split_u16(n: u16) -> (u8, u8) {
    let hi: u8 = (n >> 8) as u8;
    let lo: u8 = n as u8;
    (hi, lo)
}

pub fn r16_to_hi_lo(r: R16) -> (R8, R8) {
    match r {
        BC => (B, C),
        DE => (D, E),
        HL => (H, L),
        AF => (A, F),
        SP => panic!(),
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum R8 {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
    F,
    HLA,
}

impl TryFrom<u8> for R8 {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<R8> {
        ensure!(value < 8);
        match value {
            0 => Ok(B),
            1 => Ok(C),
            2 => Ok(D),
            3 => Ok(E),
            4 => Ok(H),
            5 => Ok(L),
            6 => Ok(HLA),
            7 => Ok(A),
            _ => bail!("invalid value"),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum R16 {
    BC,
    DE,
    HL,
    AF,
    SP,
}

#[derive(Default)]
struct InterruptFlags {
    joy: bool,
    serial: bool,
    timer: bool,
    lcd: bool,
    vblank: bool,
}

impl InterruptFlags {
    pub fn set(&mut self, flags: u8) {
        (self.joy, self.serial, self.timer, self.lcd, self.vblank) = (
            (flags >> 4) & 1 == 1,
            (flags >> 3) & 1 == 1,
            (flags >> 2) & 1 == 1,
            (flags >> 1) & 1 == 1,
            flags & 1 == 1,
        );
    }

    pub fn read(&self) -> u8 {
        (self.joy as u8) << 4
            | (self.serial as u8) << 3
            | (self.timer as u8) << 2
            | (self.lcd as u8) << 1
            | (self.vblank as u8)
    }
}
