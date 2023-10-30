use crate::cpu::instruction::Instruction;

use self::bus::Bus;

pub mod bus;
pub mod execute;
pub mod instruction;

pub struct CPU {
  pub pc: u32,
  pub r: [u32; 32],
  hi: u32,
  low: u32,
  bus: Bus,
  pipeline: [u32; 2],
  previous_pc: u32
}

impl CPU {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      pc: 0xbfc0_0000,
      previous_pc: 0,
      r: [0; 32],
      hi: 0,
      low: 0,
      bus: Bus::new(bios),
      pipeline: [0; 2]
    }
  }
  pub fn step(&mut self) {
    let next_instr = self.bus.mem_read_32(self.pc);

    let instr = self.pipeline[0];

    self.pipeline[0] = self.pipeline[1];
    self.pipeline[1] = next_instr;

    println!("executing instruction {:032b} at address {:08x}", instr, self.previous_pc);

    self.previous_pc = self.pc - 4;
    self.pc = self.pc.wrapping_add(4);

    self.execute(Instruction::new(instr));
  }

  pub fn set_reg(&mut self, rt: usize, val: u32) {
    if rt != 0 {
      self.r[rt] = val
    }
  }
}