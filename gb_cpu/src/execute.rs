//! Contains logic for CPU operation

use super::decode;
use super::{CpuInputPins, CpuOutputPins, FRegister};

impl super::Cpu {
    /// Set the output pins to fetch the memory located at the address in the PC register, and then increment the PC register.
    /// The value of the address pins is equal to the PC register *before* being incremented.
    fn fetch_byte(&mut self) -> CpuOutputPins {
        let pc = self.registers.get_pc();
        self.registers.set_pc(pc.wrapping_add(1));
        CpuOutputPins::Read { addr: pc }
    }

    /// Set the pins to write a byte to memory
    fn write_byte(&self, addr: u16, data: u8) -> CpuOutputPins {
        CpuOutputPins::Write { addr, data }
    }

    fn read_byte(&self, addr: u16) -> CpuOutputPins {
        CpuOutputPins::Read { addr }
    }

    fn nop(&self) -> CpuOutputPins {
        CpuOutputPins::Read { addr: 0 }
    }

    fn store_16_bits(&mut self, v: u16, dest: LoadDest16Bit) {
        match dest {
            LoadDest16Bit::AF => self.registers.set_af(v),
            LoadDest16Bit::BC => self.registers.set_bc(v),
            LoadDest16Bit::DE => self.registers.set_de(v),
            LoadDest16Bit::HL => self.registers.set_hl(v),
            LoadDest16Bit::SP => self.registers.set_sp(v),
        }
    }

    fn read_16_bits(&mut self, from: LoadDest16Bit) -> u16 {
        match from {
            LoadDest16Bit::AF => self.registers.get_af(),
            LoadDest16Bit::BC => self.registers.get_bc(),
            LoadDest16Bit::DE => self.registers.get_de(),
            LoadDest16Bit::HL => self.registers.get_hl(),
            LoadDest16Bit::SP => self.registers.get_sp(),
        }
    }

    /// Perform an ALU operation on the accumulator and update the flags register. The operation is chosen by:
    ///
    /// 0 = ADD, 1 = ADC, 2 = SUB, 3 = SBC, 4 = AND, 5 = XOR, 6 = OR, 7 = CP
    fn do_math(&mut self, v: u8, operation: MathOperation) {
        use MathOperation::*;
        match operation {
            Add => {
                let a = self.registers.get_a();
                let (sum, overflow) = a.overflowing_add(v);
                self.registers.set_a(sum);
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) + (v & 0x0f) >= 0x10);
                    f.set_value(FRegister::CARRY, overflow);

                    f
                })
            }
            Adc => {
                let a = self.registers.get_a() as u16;
                let v = v as u16;
                let vc = if self.registers.get_f().contains(FRegister::CARRY) {
                    v + 1
                } else {
                    v
                };
                let sum = a + vc;
                let carry = sum >= 0x100;
                let bytesum = sum as u8;

                let al = a & 0x0f;
                let vl = v & 0x0f;
                let vlc = if self.registers.get_f().contains(FRegister::CARRY) {
                    vl + 1
                } else {
                    vl
                };
                let halfcarry = vlc + al >= 0x10;
                self.registers.set_a(bytesum);
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, bytesum == 0);
                    f.set_value(FRegister::HALFCARRY, halfcarry);
                    f.set_value(FRegister::CARRY, carry);

                    f
                })
            }
            Sub => {
                let a = self.registers.get_a();
                let nv = (!v).wrapping_add(1); // Two's complement of v (makes flags easier)
                let sum = a.wrapping_add(nv);
                self.registers.set_a(sum);
                self.registers.modify_f(|mut f| {
                    f.set(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) < (v & 0x0f));
                    f.set_value(FRegister::CARRY, v > a);

                    f
                })
            }
            Sbc => {
                let a = self.registers.get_a() as u16 as i16;
                let v = v as u16 as i16;
                let vc = if self.registers.get_f().contains(FRegister::CARRY) {
                    v + 1
                } else {
                    v
                };
                let carry = vc > a;
                let sum = a - vc;
                let bytesum = sum as u16 as u8;

                let al = a & 0x0f;
                let vl = v & 0x0f;
                let vlc = if self.registers.get_f().contains(FRegister::CARRY) {
                    vl + 1
                } else {
                    vl
                };
                let halfcarry = vlc > al;
                self.registers.set_a(bytesum);
                self.registers.modify_f(|mut f| {
                    f.set(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, bytesum == 0);
                    f.set_value(FRegister::HALFCARRY, halfcarry);
                    f.set_value(FRegister::CARRY, carry);

                    f
                })
            }
            And => {
                self.registers.modify_a(|a| a & v);
                let new_a = self.registers.get_a();
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, new_a == 0);
                    f.set(FRegister::HALFCARRY);
                    f.unset(FRegister::CARRY);

                    f
                });
            }
            Xor => {
                self.registers.modify_a(|a| a ^ v);
                let new_a = self.registers.get_a();
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, new_a == 0);
                    f.unset(FRegister::HALFCARRY);
                    f.unset(FRegister::CARRY);

                    f
                });
            }
            Or => {
                self.registers.modify_a(|a| a | v);
                let new_a = self.registers.get_a();
                self.registers.modify_f(|mut f| {
                    f.unset(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, new_a == 0);
                    f.unset(FRegister::HALFCARRY);
                    f.unset(FRegister::CARRY);

                    f
                });
            }
            Cp => {
                let a = self.registers.get_a();
                let nv = (!v).wrapping_add(1); // Two's complement of v (makes flags easier)
                let sum = a.wrapping_add(nv);
                self.registers.modify_f(|mut f| {
                    f.set(FRegister::NEGATIVE);
                    f.set_value(FRegister::ZERO, sum == 0);
                    f.set_value(FRegister::HALFCARRY, (a & 0x0f) < (v & 0x0f));
                    f.set_value(FRegister::CARRY, v > a);

                    f
                })
            }
        }
    }

    fn daa(&mut self) {
        let mut f = self.registers.get_f();
        let mut a = self.registers.get_a();

        if !f.contains(FRegister::NEGATIVE) {
            if f.contains(FRegister::CARRY) || a > 0x99 {
                a = a.wrapping_add(0x60);
                f.set(FRegister::CARRY);
            }
            if f.contains(FRegister::HALFCARRY) || (a & 0x0F) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        } else {
            if f.contains(FRegister::CARRY) {
                a = a.wrapping_sub(0x60);
            }
            if f.contains(FRegister::HALFCARRY) {
                a = a.wrapping_sub(0x06);
            }
        }

        f.set_value(FRegister::ZERO, a == 0);
        f.unset(FRegister::HALFCARRY);

        self.registers.set_f(f);
        self.registers.set_a(a);
    }

    fn do_rotate_shift(&mut self, v: u8, op: RotateShiftOperation) -> u8 {
        use RotateShiftOperation::*;
        match op {
            RLC => {
                let c = v & 0x80 != 0;
                let nv = v.rotate_left(1);
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.set_value(FRegister::CARRY, c);
                    f
                });
                nv
            }
            RRC => {
                let c = v & 0x01 != 0;
                let nv = v.rotate_right(1);
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.set_value(FRegister::CARRY, c);
                    f
                });
                nv
            }
            RL => {
                let rotate_in = if self.registers.get_f().contains(FRegister::CARRY) {
                    1
                } else {
                    0
                };
                let c = (v & 0x80) != 0;
                let nv = v << 1;
                let nv = nv | rotate_in;
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.set_value(FRegister::CARRY, c);
                    f
                });
                nv
            }
            RR => {
                let rotate_in = if self.registers.get_f().contains(FRegister::CARRY) {
                    0x80
                } else {
                    0x00
                };
                let c = (v & 0x01) != 0;
                let nv = v >> 1;
                let nv = nv | rotate_in;
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.set_value(FRegister::CARRY, c);
                    f
                });
                nv
            }
            SLA => {
                let c = (v & 0x80) != 0;
                let nv = v << 1;
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.set_value(FRegister::CARRY, c);
                    f
                });
                nv
            }
            SRA => {
                let msb = v & 0x80;
                let c = (v & 0x01) != 0;
                let nv = v >> 1;
                let nv = nv | msb;
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.set_value(FRegister::CARRY, c);
                    f
                });
                nv
            }
            SWAP => {
                let lo = v & 0x0F;
                let hi = (v & 0xF0) >> 4;

                let nv = (lo << 4) | hi;
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.unset(FRegister::CARRY);
                    f
                });
                nv
            }
            SRL => {
                let c = (v & 0x01) != 0;
                let nv = v >> 1;
                let z = nv == 0;
                self.registers.modify_f(|mut f| {
                    f.set_value(FRegister::ZERO, z);
                    f.unset(FRegister::NEGATIVE);
                    f.unset(FRegister::HALFCARRY);
                    f.set_value(FRegister::CARRY, c);
                    f
                });
                nv
            }
        }
    }

    fn test_condition(&self, c: FlagCondition) -> bool {
        match c {
            FlagCondition::NZ => !self.registers.f.contains(FRegister::ZERO),
            FlagCondition::Z => self.registers.f.contains(FRegister::ZERO),
            FlagCondition::NC => !self.registers.f.contains(FRegister::CARRY),
            FlagCondition::C => self.registers.f.contains(FRegister::CARRY),
        }
    }

    pub fn runner(self) -> CpuRunner {
        CpuRunner {
            cpu: self,
            gen: Box::pin(cpu_runner_gen()),
        }
    }
}

pub struct CpuRunnerYield {
    pub pins: CpuOutputPins,
    /// Indicates that the CPU is fetching the next opcode. Used for debug purposes.
    pub is_fetch_cycle: bool,
}

type CpuRunnerGen = std::pin::Pin<
    Box<
        dyn std::ops::Coroutine<
                (super::Cpu, CpuInputPins),
                Yield = (super::Cpu, CpuRunnerYield),
                Return = !,
            > + Send
            + Sync,
    >,
>;

/// Provides a wrapper to use around the generator underneath the CPU execution logic.
pub struct CpuRunner {
    pub cpu: super::Cpu,
    gen: CpuRunnerGen,
}

impl CpuRunner {
    /// Clock the CPU by exactly one M-cycle
    pub fn clock(&mut self, pins: CpuInputPins) -> CpuRunnerYield {
        use std::ops::CoroutineState;
        match self.gen.as_mut().resume((self.cpu, pins)) {
            CoroutineState::Yielded((cpu, pins_out)) => {
                self.cpu = cpu;
                pins_out
            }
            CoroutineState::Complete(_) => unreachable!(),
        }
    }
}

impl std::fmt::Debug for CpuRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpuRunner")
            .field("Cpu", &self.cpu)
            .finish_non_exhaustive()
    }
}

/// Yields a generator containing state that will run the cpu
fn cpu_runner_gen() -> impl std::ops::Coroutine<
    (super::Cpu, CpuInputPins),
    Yield = (super::Cpu, CpuRunnerYield),
    Return = !,
> + Send
       + Sync {
    // Every `yield` here will cause the CPU to wait for one memory cycle.
    #[allow(unused_assignments)]
    move |t: (super::Cpu, CpuInputPins)| {
        let (mut cpu, mut pins) = t;
        let mut halted = false;
        let mut fetch = false;
        loop {
            macro_rules! cpu_yield {
                ($pins:expr) => {
                    let _yielded = CpuRunnerYield {
                        pins: $pins,
                        is_fetch_cycle: fetch,
                    };
                    (cpu, pins) = yield (cpu, _yielded);
                };
            }

            /// Store an 8 bit value into a register specified by the `r` table. Yields a cpu cycle on indirect HL write, unyielding otherwise.
            ///
            /// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
            macro_rules! store_8_bits {
                ($self:ident, $v:expr, $dest:expr) => {
                    match $dest {
                        LoadDest::B => $self.registers.set_b($v),
                        LoadDest::C => $self.registers.set_c($v),
                        LoadDest::D => $self.registers.set_d($v),
                        LoadDest::E => $self.registers.set_e($v),
                        LoadDest::H => $self.registers.set_h($v),
                        LoadDest::L => $self.registers.set_l($v),
                        LoadDest::IndHL => {
                            cpu_yield!($self.write_byte($self.registers.get_hl(), $v));
                        }
                        LoadDest::A => $self.registers.set_a($v),
                    }
                };
            }

            /// Read an 8 bit value from a register specified by the `r` table. Yields a cpu cycle on indirect HL read, unyielding otherwise.
            ///
            /// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
            macro_rules! read_8_bits {
                ($self:ident, $dest:expr) => {
                    match $dest {
                        LoadDest::B => $self.registers.get_b(),
                        LoadDest::C => $self.registers.get_c(),
                        LoadDest::D => $self.registers.get_d(),
                        LoadDest::E => $self.registers.get_e(),
                        LoadDest::H => $self.registers.get_h(),
                        LoadDest::L => $self.registers.get_l(),
                        LoadDest::IndHL => {
                            cpu_yield!($self.read_byte($self.registers.get_hl()));
                            pins.data
                        }
                        LoadDest::A => $self.registers.get_a(),
                    }
                };
            }

            // Handle interrupts
            let interrupt = if pins.interrupt_40h {
                Some(0x40)
            } else if pins.interrupt_48h {
                Some(0x48)
            } else if pins.interrupt_50h {
                Some(0x50)
            } else if pins.interrupt_58h {
                Some(0x58)
            } else if pins.interrupt_60h {
                Some(0x60)
            } else {
                None
            };

            if let Some(vector) = interrupt {
                halted = false;
                if cpu.ime {
                    // Interrupt Service Routine (5 clock cycles)
                    // https://gbdev.io/pandocs/Interrupts.html#interrupt-handling

                    // Normally, these two cycles would both be NOPs, but in this architecture we
                    // have to take advantage of these cycles to reset the IF flag.
                    cpu_yield!(cpu.read_byte(0xFF0F));
                    let interrupt_flag = pins.data;
                    let if_mask = !(1 << ((vector - 0x40) / 8));
                    cpu_yield!(cpu.write_byte(0xFF0F, interrupt_flag & if_mask));

                    let pc = cpu.registers.get_pc();
                    let pc_lo = (pc & 0xFF) as u8;
                    let pc_hi = (pc >> 8) as u8;

                    // Push PC onto the stack
                    cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                    cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_hi));
                    cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                    cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_lo));

                    cpu.registers.set_pc(vector);

                    cpu.ime = false;

                    cpu_yield!(cpu.nop());
                }
            }

            // If the CPU is halted, stop processing instructions, and wait for an interrupt to wake up the CPU.
            if halted {
                cpu_yield!(cpu.nop());
                continue;
            }

            // Fetch
            fetch = true;
            cpu_yield!(cpu.fetch_byte());
            fetch = false;
            let opcode = super::decode::Opcode(pins.data);

            // Decode & execute
            //
            // Note: `continue` will immediately jump back to the instruction fetch logic.
            // This is intentional and is part of the fetch/execute overlap optimization done on the real cpu.
            //
            // Macros will be used here to abstract over common operations that may yield. We have to do this because
            // rust generators have no equivalent to python's `yield from`
            match opcode.x() {
                0 => match opcode.z() {
                    0 => match opcode.y() {
                        0 => continue, // NOP
                        1 => {
                            // LD (nn), SP
                            cpu_yield!(cpu.fetch_byte());
                            let low = pins.data;
                            cpu_yield!(cpu.fetch_byte());
                            let high = pins.data;

                            let addr = ((high as u16) << 8) | (low as u16);

                            let sp = cpu.registers.get_sp();
                            let sp_lo = (sp & 0xFF) as u8;
                            let sp_hi = (sp >> 8) as u8;

                            cpu_yield!(cpu.write_byte(addr, sp_lo));
                            cpu_yield!(cpu.write_byte(addr + 1, sp_hi));
                            continue;
                        }
                        2 => {
                            // STOP
                            // STOP is too wierd. just alias it to HALT for now
                            halted = true;
                        }
                        3 => {
                            // JR d
                            cpu_yield!(cpu.fetch_byte());
                            let offset = pins.data as i8 as i16;
                            let pc = cpu.registers.get_pc() as i16;
                            let new_pc = (pc + offset) as u16;
                            cpu.registers.set_pc(new_pc);

                            cpu_yield!(cpu.nop());

                            continue;
                        }
                        y @ 4..=7 => {
                            // JR d
                            let cond = decode::cc(y - 4);
                            cpu_yield!(cpu.fetch_byte());
                            let offset = pins.data as i8 as i16;

                            if cpu.test_condition(cond) {
                                let pc = cpu.registers.get_pc() as i16;
                                let new_pc = (pc + offset) as u16;
                                cpu.registers.set_pc(new_pc);

                                cpu_yield!(cpu.nop());

                                continue;
                            } else {
                                continue;
                            }
                        }
                        _ => unreachable!(),
                    },
                    1 if opcode.q() == 0 => {
                        // 16-bit LD
                        let dst = decode::rp(opcode.p());

                        cpu_yield!(cpu.fetch_byte());
                        let low = pins.data;
                        cpu_yield!(cpu.fetch_byte());
                        let high = pins.data;

                        let v = ((high as u16) << 8) | (low as u16);
                        cpu.store_16_bits(v, dst);
                    }
                    1 if opcode.q() == 1 => {
                        // 16-bit ADD
                        let from = decode::rp(opcode.p());

                        let hl = cpu.registers.get_hl();
                        let addend = cpu.read_16_bits(from);

                        let half_carry = (((hl & 0x0FFF) + (addend & 0x0FFF)) & 0x1000) != 0;
                        let (new_hl, carry) = hl.overflowing_add(addend);

                        // This instruction takes an extra cycle
                        cpu_yield!(cpu.nop());

                        cpu.registers.modify_f(|mut f| {
                            f.unset(FRegister::NEGATIVE);
                            f.set_value(FRegister::HALFCARRY, half_carry);
                            f.set_value(FRegister::CARRY, carry);
                            f
                        });
                        cpu.registers.set_hl(new_hl);
                        continue;
                    }
                    2 if opcode.q() == 0 => {
                        // LD to memory
                        let addr = match opcode.p() {
                            0 => cpu.registers.get_bc(),
                            1 => cpu.registers.get_de(),
                            2 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_add(1));
                                a
                            }
                            3 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_sub(1));
                                a
                            }
                            _ => unreachable!(),
                        };

                        cpu_yield!(cpu.write_byte(addr, cpu.registers.get_a()));
                    }
                    2 if opcode.q() == 1 => {
                        // LD from memory
                        let addr = match opcode.p() {
                            0 => cpu.registers.get_bc(),
                            1 => cpu.registers.get_de(),
                            2 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_add(1));
                                a
                            }
                            3 => {
                                let a = cpu.registers.get_hl();
                                cpu.registers.modify_hl(|hl| hl.wrapping_sub(1));
                                a
                            }
                            _ => unreachable!(),
                        };

                        cpu_yield!(cpu.read_byte(addr));
                        cpu.registers.set_a(pins.data);
                        continue;
                    }
                    3 if opcode.q() == 0 => {
                        // 16 bit INC
                        let dst = decode::rp(opcode.p());

                        let v = cpu.read_16_bits(dst);
                        let nv = v.wrapping_add(1);
                        // Pause for a cycle
                        cpu_yield!(cpu.nop());
                        cpu.store_16_bits(nv, dst);
                        continue;
                    }
                    3 if opcode.q() == 1 => {
                        // 16 bit DEC
                        let dst = decode::rp(opcode.p());

                        let v = cpu.read_16_bits(dst);
                        let nv = v.wrapping_sub(1);
                        // Pause for a cycle
                        cpu_yield!(cpu.nop());
                        cpu.store_16_bits(nv, dst);
                        continue;
                    }
                    4 => {
                        // 8 bit INC
                        let dst = decode::r(opcode.y());

                        let v = read_8_bits!(cpu, dst);
                        let nv = v.wrapping_add(1);
                        let z = nv == 0;
                        // a half carry can only happen when the lower nybble is 0xF
                        let hc = (v & 0xf) == 0xf;
                        cpu.registers.modify_f(|mut f| {
                            f.set_value(FRegister::ZERO, z);
                            f.unset(FRegister::NEGATIVE);
                            f.set_value(FRegister::HALFCARRY, hc);
                            f
                        });
                        store_8_bits!(cpu, nv, dst);
                        continue;
                    }
                    5 => {
                        // 8 bit DEC
                        let dst = decode::r(opcode.y());

                        let v = read_8_bits!(cpu, dst);
                        let nv = v.wrapping_sub(1); // equiv. to wrapping_add(255)
                        let z = nv == 0;
                        // a half carry will always happen when the lower nybble equals 0
                        let hc = (v & 0xf) == 0x0;
                        cpu.registers.modify_f(|mut f| {
                            f.set_value(FRegister::ZERO, z);
                            f.set(FRegister::NEGATIVE);
                            f.set_value(FRegister::HALFCARRY, hc);
                            f
                        });
                        store_8_bits!(cpu, nv, dst);
                        continue;
                    }
                    6 => {
                        // LD from immediate
                        let dst = decode::r(opcode.y());

                        cpu_yield!(cpu.fetch_byte());
                        store_8_bits!(cpu, pins.data, dst);
                        continue;
                    }
                    7 => match opcode.y() {
                        0 => {
                            // RLCA
                            let a = cpu.registers.get_a();
                            let na = cpu.do_rotate_shift(a, RotateShiftOperation::RLC);
                            cpu.registers.set_a(na);
                            cpu.registers.modify_f(|mut f| {
                                f.unset(f & !FRegister::CARRY);
                                f
                            });
                        }
                        1 => {
                            // RRCA
                            let a = cpu.registers.get_a();
                            let na = cpu.do_rotate_shift(a, RotateShiftOperation::RRC);
                            cpu.registers.set_a(na);
                            cpu.registers.modify_f(|mut f| {
                                f.unset(f & !FRegister::CARRY);
                                f
                            });
                        }
                        2 => {
                            // RLA
                            let a = cpu.registers.get_a();
                            let na = cpu.do_rotate_shift(a, RotateShiftOperation::RL);
                            cpu.registers.set_a(na);
                            cpu.registers.modify_f(|mut f| {
                                f.unset(f & !FRegister::CARRY);
                                f
                            });
                        }
                        3 => {
                            // RRA
                            let a = cpu.registers.get_a();
                            let na = cpu.do_rotate_shift(a, RotateShiftOperation::RR);
                            cpu.registers.set_a(na);
                            cpu.registers.modify_f(|mut f| {
                                f.unset(f & !FRegister::CARRY);
                                f
                            });
                        }
                        4 => {
                            // DAA
                            cpu.daa();
                            continue;
                        }
                        5 => {
                            // CPL
                            cpu.registers.modify_a(|a| !a);
                            cpu.registers.modify_f(|mut f| {
                                f.set(FRegister::NEGATIVE);
                                f.set(FRegister::HALFCARRY);
                                f
                            });
                            continue;
                        }
                        6 => {
                            // SCF
                            cpu.registers.modify_f(|mut f| {
                                f.unset(FRegister::NEGATIVE);
                                f.unset(FRegister::HALFCARRY);
                                f.set(FRegister::CARRY);
                                f
                            });
                            continue;
                        }
                        7 => {
                            // CCF
                            cpu.registers.modify_f(|mut f| {
                                f.unset(FRegister::NEGATIVE);
                                f.unset(FRegister::HALFCARRY);
                                f.set_value(FRegister::CARRY, !f.contains(FRegister::CARRY));
                                f
                            });
                            continue;
                        }
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                },
                1 if opcode.z() == 6 && opcode.y() == 6 => {
                    // HALT
                    halted = true;
                    continue;
                }
                1 => {
                    // 8-bit register-to-register LD
                    let dst = decode::r(opcode.y());
                    let from = decode::r(opcode.z());

                    let v = read_8_bits!(cpu, from);
                    store_8_bits!(cpu, v, dst);
                    continue;
                }
                2 => {
                    let op = decode::alu(opcode.y());
                    let reg = decode::r(opcode.z());

                    let v = read_8_bits!(cpu, reg);
                    cpu.do_math(v, op);
                    continue;
                }
                3 => match opcode.z() {
                    0 => match opcode.y() {
                        y @ 0..=3 => {
                            // RET cc
                            // Pause for a cycle
                            cpu_yield!(cpu.nop());

                            if cpu.test_condition(decode::cc(y)) {
                                cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                                let pc_lo = pins.data;
                                cpu.registers.modify_sp(|sp| sp.wrapping_add(1));
                                cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                                let pc_hi = pins.data;
                                cpu.registers.modify_sp(|sp| sp.wrapping_add(1));
                                let pc = ((pc_hi as u16) << 8) | (pc_lo as u16);
                                // Pause for a cycle
                                cpu_yield!(cpu.nop());
                                cpu.registers.set_pc(pc);
                                continue;
                            } else {
                                continue;
                            }
                        }
                        4 => {
                            // LDH (n), A
                            cpu_yield!(cpu.fetch_byte());
                            let n = pins.data;
                            let addr = 0xFF00 + (n as u16);
                            let v = cpu.registers.get_a();
                            cpu_yield!(cpu.write_byte(addr, v));
                            continue;
                        }
                        5 => {
                            // ADD SP, n
                            cpu_yield!(cpu.fetch_byte());
                            let n = pins.data as i8 as i16 as u16;
                            let sp = cpu.registers.get_sp();
                            let v = sp.wrapping_add(n);
                            let carry = (sp & 0xff) + (n & 0xff) >= 0x100;
                            let halfcarry = (sp & 0x0f) + (n & 0x0f) >= 0x10;
                            // Pause
                            cpu_yield!(cpu.nop());
                            cpu.registers.set_sp(v);
                            cpu.registers.modify_f(|_| {
                                let mut f = FRegister::EMPTY;
                                f.set_value(FRegister::CARRY, carry);
                                f.set_value(FRegister::HALFCARRY, halfcarry);
                                f
                            });
                            // Pause again for some reason
                            cpu_yield!(cpu.nop());
                            continue;
                        }
                        6 => {
                            // LDH A, (n)
                            cpu_yield!(cpu.fetch_byte());
                            let n = pins.data;
                            let addr = 0xFF00 + (n as u16);
                            cpu_yield!(cpu.read_byte(addr));
                            cpu.registers.set_a(pins.data);
                            continue;
                        }
                        7 => {
                            // LD HL, SP+d
                            cpu_yield!(cpu.fetch_byte());
                            let n = pins.data as i8 as i16 as u16;
                            let sp = cpu.registers.get_sp();
                            let v = sp.wrapping_add(n);
                            let carry = (sp & 0xff) + (n & 0xff) >= 0x100;
                            let halfcarry = (sp & 0x0f) + (n & 0x0f) >= 0x10;
                            // Pause
                            cpu_yield!(cpu.nop());
                            cpu.registers.set_hl(v);
                            cpu.registers.modify_f(|_| {
                                let mut f = FRegister::EMPTY;
                                f.set_value(FRegister::CARRY, carry);
                                f.set_value(FRegister::HALFCARRY, halfcarry);
                                f
                            });
                            continue;
                        }
                        _ => unreachable!(),
                    },
                    1 if opcode.q() == 0 => {
                        // POP
                        let dst = decode::rp2(opcode.p());

                        cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                        let low = pins.data;
                        cpu.registers.modify_sp(|sp| sp.wrapping_add(1));
                        cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                        let high = pins.data;
                        cpu.registers.modify_sp(|sp| sp.wrapping_add(1));

                        let v = ((high as u16) << 8) | (low as u16);
                        cpu.store_16_bits(v, dst);
                        continue;
                    }
                    1 if opcode.q() == 1 => match opcode.p() {
                        0 => {
                            // RET
                            cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                            let pc_lo = pins.data;
                            cpu.registers.modify_sp(|sp| sp.wrapping_add(1));
                            cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                            let pc_hi = pins.data;
                            cpu.registers.modify_sp(|sp| sp.wrapping_add(1));

                            // Pause for a cycle
                            cpu_yield!(cpu.nop());

                            let pc = ((pc_hi as u16) << 8) | (pc_lo as u16);
                            cpu.registers.set_pc(pc);
                            continue;
                        }
                        1 => {
                            // RETI
                            cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                            let pc_lo = pins.data;
                            cpu.registers.modify_sp(|sp| sp.wrapping_add(1));
                            cpu_yield!(cpu.read_byte(cpu.registers.get_sp()));
                            let pc_hi = pins.data;
                            cpu.registers.modify_sp(|sp| sp.wrapping_add(1));

                            // Pause for a cycle
                            cpu_yield!(cpu.nop());

                            let pc = ((pc_hi as u16) << 8) | (pc_lo as u16);
                            cpu.registers.set_pc(pc);
                            cpu.ime = true;
                            continue;
                        }
                        2 => {
                            // JP HL
                            cpu.registers.set_pc(cpu.registers.get_hl());
                            continue;
                        }
                        3 => {
                            // LD SP, HL
                            cpu.registers.set_sp(cpu.registers.get_hl());
                            cpu_yield!(cpu.nop());
                            continue;
                        }
                        _ => unreachable!(),
                    },
                    2 => match opcode.y() {
                        y @ 0..=3 => {
                            // JP c nn
                            let condition = decode::cc(y);

                            cpu_yield!(cpu.fetch_byte());
                            let low = pins.data;
                            cpu_yield!(cpu.fetch_byte());
                            let high = pins.data;

                            let addr = ((high as u16) << 8) | (low as u16);

                            if cpu.test_condition(condition) {
                                cpu.registers.set_pc(addr);
                                // Pause for a cycle
                                cpu_yield!(cpu.nop());
                            } else {
                                continue;
                            }
                        }
                        4 => {
                            // LD (C), A
                            let addr = 0xFF00 + (cpu.registers.get_c() as u16);
                            let v = cpu.registers.get_a();
                            cpu_yield!(cpu.write_byte(addr, v));
                            continue;
                        }
                        6 => {
                            // LD A, (C)
                            let addr = 0xFF00 + (cpu.registers.get_c() as u16);
                            cpu_yield!(cpu.read_byte(addr));
                            let v = pins.data;
                            cpu.registers.set_a(v);
                            continue;
                        }
                        5 => {
                            // LD (nn), A
                            cpu_yield!(cpu.fetch_byte());
                            let low = pins.data;
                            cpu_yield!(cpu.fetch_byte());
                            let high = pins.data;

                            let addr = ((high as u16) << 8) | (low as u16);
                            let v = cpu.registers.get_a();
                            cpu_yield!(cpu.write_byte(addr, v));
                            continue;
                        }
                        7 => {
                            // LD A, (nn)
                            cpu_yield!(cpu.fetch_byte());
                            let low = pins.data;
                            cpu_yield!(cpu.fetch_byte());
                            let high = pins.data;

                            let addr = ((high as u16) << 8) | (low as u16);
                            cpu_yield!(cpu.read_byte(addr));
                            let v = pins.data;
                            cpu.registers.set_a(v);
                            continue;
                        }
                        _ => unreachable!(),
                    },
                    3 => match opcode.y() {
                        0 => {
                            // JP nn
                            cpu_yield!(cpu.fetch_byte());
                            let low = pins.data;
                            cpu_yield!(cpu.fetch_byte());
                            let high = pins.data;

                            let addr = ((high as u16) << 8) | (low as u16);
                            cpu.registers.set_pc(addr);
                            continue;
                        }
                        1 => {
                            // CB Prefix

                            cpu_yield!(cpu.fetch_byte());
                            let opcode = decode::Opcode(pins.data);

                            let dest = decode::r(opcode.z());
                            let v = read_8_bits!(cpu, dest);

                            match opcode.x() {
                                0 => {
                                    let nv = cpu.do_rotate_shift(v, decode::rot(opcode.y()));
                                    store_8_bits!(cpu, nv, dest);
                                }
                                1 => {
                                    // BIT
                                    let n = opcode.y();
                                    let z = v & (1 << n) == 0;
                                    cpu.registers.modify_f(|mut f| {
                                        f.set_value(FRegister::ZERO, z);
                                        f.unset(FRegister::NEGATIVE);
                                        f.set(FRegister::HALFCARRY);
                                        f
                                    });
                                }
                                2 => {
                                    // RES
                                    let n = opcode.y();
                                    let nv = v & !(1 << n);
                                    store_8_bits!(cpu, nv, dest);
                                }
                                3 => {
                                    // SET
                                    let n = opcode.y();
                                    let nv = v | (1 << n);
                                    store_8_bits!(cpu, nv, dest);
                                }
                                _ => unreachable!(),
                            }

                            continue;
                        }
                        6 => {
                            // DI
                            cpu.ime = false;
                            continue;
                        }
                        7 => {
                            // EI
                            cpu.ime = true;
                            continue;
                        }
                        _ => panic!("Unidentified opcode: {:?}, {:X?}", cpu, opcode),
                    },
                    4 => match opcode.y() {
                        y @ 0..=3 => {
                            // CALL cc, nn
                            cpu_yield!(cpu.fetch_byte());
                            let low = pins.data;
                            cpu_yield!(cpu.fetch_byte());
                            let high = pins.data;

                            let addr = ((high as u16) << 8) | (low as u16);

                            if cpu.test_condition(decode::cc(y)) {
                                let pc = cpu.registers.get_pc();
                                let pc_lo = (pc & 0xFF) as u8;
                                let pc_hi = (pc >> 8) as u8;

                                cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                                cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_hi));
                                cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                                cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_lo));

                                cpu.registers.set_pc(addr);
                                // Pause for a cycle
                                cpu_yield!(cpu.nop());

                                continue;
                            } else {
                                continue;
                            }
                        }
                        4..=7 => panic!(),
                        _ => unreachable!(),
                    },
                    5 if opcode.q() == 0 => {
                        // PUSH
                        let from = decode::rp2(opcode.p());
                        let v = cpu.read_16_bits(from);

                        cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                        let high = (v >> 8) as u8;
                        cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), high));
                        cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                        let low = (v & 0x00ff) as u8;
                        cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), low));
                        continue;
                    }
                    5 if opcode.q() == 1 => match opcode.p() {
                        0 => {
                            // CALL nn
                            cpu_yield!(cpu.fetch_byte());
                            let low = pins.data;
                            cpu_yield!(cpu.fetch_byte());
                            let high = pins.data;

                            let addr = ((high as u16) << 8) | (low as u16);

                            let pc = cpu.registers.get_pc();
                            let pc_lo = (pc & 0xFF) as u8;
                            let pc_hi = (pc >> 8) as u8;

                            cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                            cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_hi));
                            cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                            cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_lo));

                            cpu.registers.set_pc(addr);
                            // Pause for a cycle
                            cpu_yield!(cpu.nop());

                            continue;
                        }
                        1..=3 => panic!(),
                        _ => unreachable!(),
                    },
                    6 => {
                        let operation = decode::alu(opcode.y());

                        cpu_yield!(cpu.fetch_byte());
                        let n = pins.data;

                        cpu.do_math(n, operation);
                        continue;
                    }
                    7 => {
                        // RST
                        let vector = opcode.y() * 8;
                        let addr = vector as u16;

                        let pc = cpu.registers.get_pc();
                        let pc_lo = (pc & 0xFF) as u8;
                        let pc_hi = (pc >> 8) as u8;

                        cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                        cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_hi));
                        cpu.registers.modify_sp(|sp| sp.wrapping_sub(1));
                        cpu_yield!(cpu.write_byte(cpu.registers.get_sp(), pc_lo));

                        cpu.registers.set_pc(addr);
                        // Pause for a cycle
                        cpu_yield!(cpu.nop());
                        continue;
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOperation {
    Add = 0,
    Adc = 1,
    Sub = 2,
    Sbc = 3,
    And = 4,
    Xor = 5,
    Or = 6,
    Cp = 7,
}

/// 8 bit registers specified by the `r` table.
///
/// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadDest {
    B,
    C,
    D,
    E,
    H,
    L,
    IndHL,
    A,
}

/// 16 bit register pairs used by the `rp` and `rp2` tables.
///
/// See https://gb-archive.github.io/salvage/decoding_gbz80_opcodes/Decoding%20Gamboy%20Z80%20Opcodes.html
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadDest16Bit {
    AF,
    BC,
    DE,
    HL,
    SP,
}

pub enum FlagCondition {
    NZ,
    Z,
    NC,
    C,
}

#[allow(clippy::upper_case_acronyms)]
pub enum RotateShiftOperation {
    RLC,
    RRC,
    RL,
    RR,
    SLA,
    SRA,
    SWAP,
    SRL,
}
