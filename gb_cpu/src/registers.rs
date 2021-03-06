use paste::paste;
use std::{
    fmt::Debug,
    ops::{BitAnd, BitOr, BitOrAssign, Not},
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Registers {
    pub a: u8,
    pub f: FRegister,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Debug for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registers")
            .field("A", &format_args!("{:02X}", self.a))
            .field("B", &format_args!("{:02X}", self.b))
            .field("C", &format_args!("{:02X}", self.c))
            .field("D", &format_args!("{:02X}", self.d))
            .field("E", &format_args!("{:02X}", self.e))
            .field("H", &format_args!("{:02X}", self.h))
            .field("L", &format_args!("{:02X}", self.l))
            .field("SP", &format_args!("{:04X}", self.sp))
            .field("PC", &format_args!("{:04X}", self.pc))
            .field("F", &self.f)
            .finish()
    }
}

macro_rules! reg_setters_and_getters {
        ($($reg:ident: $type: ident),+) => {
            $(
                paste! {
                    pub fn [<get_ $reg>](&self) -> $type {
                        self.$reg
                    }

                    pub fn [<set_ $reg>](&mut self, v: $type) {
                        self.$reg = v
                    }

                    pub fn [<modify_ $reg>]<F: FnOnce($type) -> $type>(&mut self, f: F) {
                        self.$reg = f(self.$reg)
                    }
                }
            )+
        };
    }

macro_rules! combined_registers {
    ($combined:ident, $low:ident, $high:ident) => {
        paste! {
            pub fn [<get_ $combined>](&self) -> u16 {
                (self.$high as u16) << 8 | self.$low as u16
            }

            pub fn [<set_ $combined>](&mut self, v: u16) {
                self.$high = (v >> 8) as u8;
                self.$low = (v&0xFF) as u8;
            }

            pub fn [<modify_ $combined>]<F: FnOnce(u16) -> u16>(&mut self, f: F) {
                self.[<set_ $combined>](f(self.[<get_ $combined>]()));
            }
        }
    };
}

impl Registers {
    #![allow(dead_code)]
    reg_setters_and_getters!(
        a: u8,
        f: FRegister,
        b: u8,
        c: u8,
        d: u8,
        e: u8,
        h: u8,
        l: u8,
        sp: u16,
        pc: u16
    );

    pub fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | u8::from(self.f) as u16
    }

    pub fn set_af(&mut self, v: u16) {
        self.a = (v >> 8) as u8;
        self.f = ((v & 0xFF) as u8).into();
    }

    pub fn modify_af<F: FnOnce(u16) -> u16>(&mut self, f: F) {
        self.set_af(f(self.get_af()));
    }

    combined_registers!(bc, c, b);
    combined_registers!(de, e, d);
    combined_registers!(hl, l, h);
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct FRegister(u8);

impl FRegister {
    pub const EMPTY: FRegister = FRegister(0);
    pub const ZERO: FRegister = FRegister(0x80);
    pub const NEGATIVE: FRegister = FRegister(0x40);
    pub const HALFCARRY: FRegister = FRegister(0x20);
    pub const CARRY: FRegister = FRegister(0x10);

    /// Returns true if any flags in the parameter are set in this value, and false otherwise
    #[inline(always)]
    pub fn contains(self, other: FRegister) -> bool {
        self.0 & other.0 != 0
    }

    /// Equivalent to `self = self | other`
    #[inline(always)]
    pub fn set(&mut self, other: FRegister) {
        *self |= other
    }

    /// Equivalent to `self = self & !other`
    #[inline(always)]
    pub fn unset(&mut self, other: FRegister) {
        *self = *self & !other
    }

    /// Equivalent to `if value { self.set(flags) } else { self.unset(flags) }`
    #[inline(always)]
    pub fn set_value(&mut self, flags: FRegister, value: bool) {
        if value {
            self.set(flags)
        } else {
            self.unset(flags)
        }
    }
}

impl BitOr for FRegister {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        FRegister(self.0 | rhs.0)
    }
}

impl BitOrAssign for FRegister {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs
    }
}

impl BitAnd for FRegister {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        FRegister(self.0 & rhs.0)
    }
}

impl Not for FRegister {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        FRegister((!self.0) & 0xF0)
    }
}

impl From<u8> for FRegister {
    fn from(v: u8) -> Self {
        FRegister(v & 0xF0)
    }
}

impl From<FRegister> for u8 {
    fn from(reg: FRegister) -> u8 {
        reg.0
    }
}

impl Debug for FRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if self.contains(FRegister::ZERO) {
                "Z"
            } else {
                "-"
            }
        )?;
        write!(
            f,
            "{}",
            if self.contains(FRegister::NEGATIVE) {
                "N"
            } else {
                "-"
            }
        )?;
        write!(
            f,
            "{}",
            if self.contains(FRegister::HALFCARRY) {
                "H"
            } else {
                "-"
            }
        )?;
        write!(
            f,
            "{}",
            if self.contains(FRegister::CARRY) {
                "C"
            } else {
                "-"
            }
        )?;
        Ok(())
    }
}
