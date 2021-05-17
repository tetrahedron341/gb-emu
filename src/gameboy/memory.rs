use crate::cpu::{CpuInputPins, CpuOutputPins};

pub struct Memory {
    work_ram_1: [u8; 0x1000],
    work_ram_2: [u8; 0x1000],
    high_ram: [u8; 0x7f],
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            work_ram_1: [0; 0x1000],
            work_ram_2: [0; 0x1000],
            high_ram: [0; 0x7f],
        }
    }

    fn address_is_in_range(addr: u16) -> bool {
        match addr {
            0xC000..=0xDFFF => true,
            0xFF80..=0xFFFE => true,
            _ => false,
        }
    }
}

impl std::ops::Index<u16> for Memory {
    type Output = u8;
    fn index(&self, index: u16) -> &Self::Output {
        match index {
            0xC000..=0xCFFF => &self.work_ram_1[(index - 0xC000) as usize],
            0xD000..=0xDFFF => &self.work_ram_2[(index - 0xD000) as usize],
            0xFF80..=0xFFFE => &self.high_ram[(index - 0xFF80) as usize],
            _ => panic!("Out of bounds: {}", index),
        }
    }
}

impl std::ops::IndexMut<u16> for Memory {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        match index {
            0xC000..=0xCFFF => &mut self.work_ram_1[(index - 0xC000) as usize],
            0xD000..=0xDFFF => &mut self.work_ram_2[(index - 0xD000) as usize],
            0xFF80..=0xFFFE => &mut self.high_ram[(index - 0xFF80) as usize],
            _ => panic!("Out of bounds: {}", index),
        }
    }
}

impl super::Chip for Memory {
    fn chip_select(&self, addr: u16) -> bool {
        Self::address_is_in_range(addr)
    }

    fn clock(&mut self, input: CpuOutputPins) -> CpuInputPins {
        match input {
            CpuOutputPins::Read { addr } => {
                debug_assert!(Self::address_is_in_range(addr));

                CpuInputPins {
                    data: self[addr],
                    ..Default::default()
                }
            }
            CpuOutputPins::Write { addr, data } => {
                debug_assert!(Self::address_is_in_range(addr));
                self[addr] = data;
                Default::default()
            }
        }
    }

    fn clock_unselected(&mut self) {}
}
