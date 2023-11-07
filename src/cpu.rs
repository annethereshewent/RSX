use crate::cpu::instruction::Instruction;

use self::bus::Bus;

pub mod bus;
pub mod execute;
pub mod instruction;
pub mod dma;

pub enum Cause {
  LoadAddressError = 0x4,
  StoreAddressError = 0x5,
  SysCall = 0x8,
  Break = 0x9,
  IllegalInstruction = 0xa,
  CoprocessorError = 0xb,
  Overflow = 0xc

}

pub struct COP0 {
  pub sr: u32,
  pub cause: u32,
  pub epc: u32
}

impl COP0 {
  pub fn bev(&self) -> bool {
    (self.sr >> 22) & 0b1 == 1
  }

  pub fn is_cache_disabled(&self) -> bool {
    self.sr & 0x10000 == 0
  }

  pub fn enter_exception(&mut self, cause: Cause) -> u32 {
    let exception_address: u32 = if self.bev() {
      0xbfc0_0180
    } else {
      0x8000_0080
    };

    let mode = self.sr & 0x3f;
    self.sr &=  !0x3f;
    self.sr |= (mode << 2) & 0x3f;

    self.cause &= !0x7c;
    self.cause |= (cause as u32) << 2;

    exception_address
  }

  pub fn return_from_exception(&mut self) {
    let mode = self.sr & 0x3f;
    self.sr &= !0xf;
    self.sr |= mode >> 2;
  }
}

pub struct CPU {
  pub pc: u32,
  pub next_pc: u32,
  current_pc: u32,
  pub r: [u32; 32],
  cop0: COP0,
  branch: bool,
  delay_slot: bool,
  hi: u32,
  low: u32,
  pub bus: Bus,
  load: Option<(usize, u32)>
}

impl CPU {
  pub fn new(bios: Vec<u8>) -> Self {
    Self {
      pc: 0xbfc0_0000,
      next_pc: 0xbfc0_0004,
      current_pc: 0xbfc0_0000,
      r: [0; 32],
      hi: 0,
      low: 0,
      bus: Bus::new(bios),
      load: None,
      branch: false,
      delay_slot: false,
      cop0: COP0 {
        sr: 0,
        cause: 0,
        epc: 0
      }
    }
  }

  pub fn exception(&mut self, cause: Cause) {
    let exception_address = self.cop0.enter_exception(cause);

    self.cop0.epc = self.current_pc;

    if self.delay_slot {
      self.cop0.epc = self.cop0.epc.wrapping_sub(4);
      self.cop0.cause |= 1 << 31;
    } else {
      self.cop0.cause &= !(1 << 31);
    }

    self.pc = exception_address;
    self.next_pc = self.pc.wrapping_add(4);
  }

  pub fn step(&mut self) {
    self.current_pc = self.pc;

    if self.current_pc & 0b11 != 0 {
      self.exception(Cause::LoadAddressError);
      return;
    }

    let instr = self.bus.mem_read_32(self.pc);

    // println!("executing instruction {:032b} at address {:08x}", instr, self.current_pc);

    self.delay_slot = self.branch;
    self.branch = false;

    self.pc = self.next_pc;
    self.next_pc = self.next_pc.wrapping_add(4);

    self.execute(Instruction::new(instr));
  }

  pub fn set_reg(&mut self, rt: usize, val: u32) {
    if rt != 0 {
      self.r[rt] = val;
    }
  }
}