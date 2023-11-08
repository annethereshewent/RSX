use crate::{cpu::instruction::Instruction, gpu::{CYCLES_PER_SCANLINE, NUM_SCANLINES_PER_FRAME, GPU_FREQUENCY}};

use self::{bus::Bus, scheduler::Schedulable, dma::{DMA, dma_channel_control_register::SyncMode}};

pub mod bus;
pub mod execute;
pub mod instruction;
pub mod dma;
pub mod scheduler;

// 33.868MHZ
pub const CPU_FREQUENCY: f64 = 33_868_800.0;

pub const CYCLES_PER_FRAME: i64 = ((CYCLES_PER_SCANLINE * NUM_SCANLINES_PER_FRAME) as f64 * (CPU_FREQUENCY / GPU_FREQUENCY)) as i64;

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
  load: Option<(usize, u32, u16)>,
  free_cycles: [u16; 32],
  free_cycles_reg: usize,
  dma: DMA
}

impl CPU {
  pub fn new(bios: Vec<u8>) -> Self {
    println!("the cycles per frame is {CYCLES_PER_FRAME}");
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
      },
      free_cycles: [0; 32],
      free_cycles_reg: 0,
      dma: DMA::new()
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
    if self.dma.is_active() {
      if self.dma.in_gap() {
        self.dma.tick_gap(&mut self.bus.scheduler);

        if !self.dma.chopping_enabled() {
          return;
        }
      } else {
        let count = self.dma.tick(&mut self.bus);
        self.bus.scheduler.tick(count);
        return;
      }
    }

    self.current_pc = self.pc;

    if self.current_pc & 0b11 != 0 {
      self.exception(Cause::LoadAddressError);
      return;
    }

    let instr = self.fetch_instruction();

    // println!("executing instruction {:032b} at address {:08x}", instr, self.current_pc);

    self.delay_slot = self.branch;
    self.branch = false;

    self.pc = self.next_pc;
    self.next_pc = self.next_pc.wrapping_add(4);

    self.tick_instruction();

    self.execute(Instruction::new(instr));
  }

  pub fn set_reg(&mut self, rt: usize, val: u32) {
    if rt != 0 {
      self.r[rt] = val;
    }
  }

  pub fn fetch_instruction(&mut self) -> u32 {
    self.bus.scheduler.tick(4);

    // TODO: add caching code later

    self.bus.mem_read_32(self.pc, false)
  }

  pub fn store_32(&mut self, address: u32, value: u32) {
    let address = Bus::translate_address(address);

    match address {
      0x1f80_1080..=0x1f80_10ff => self.dma.write(address, value),
      _ => self.bus.mem_write_32(address, value)
    }
  }

  // TODO: refactor this into just one method
  pub fn load_32(&mut self, address: u32) -> (u32, u16) {
    let previous_cycles = self.synchronize_and_get_current_cycles();

    let address = Bus::translate_address(address);

    let result = match address {
      0x1f80_1080..=0x1f80_10ff => self.dma.read(address),
      _ => self.bus.mem_read_32(address, true)
    };

    let duration = (self.bus.scheduler.cycles - previous_cycles) as u16;

    (result, duration)
  }

  pub fn load_16(&mut self, address: u32) -> (u16, u16) {
    let previous_cycles = self.synchronize_and_get_current_cycles();

    let result = self.bus.mem_read_16(address);

    let duration = (self.bus.scheduler.cycles - previous_cycles) as u16;

    (result, duration)
  }

  pub fn synchronize_and_get_current_cycles(&mut self) -> i64 {
    self.synchronize_load();

    if self.load.is_none() {
      self.bus.scheduler.tick(2);
    }

    let previous_cycles = self.bus.scheduler.cycles;

    // this is the delay to complete the load. TODO: check if command is LWC, as that changes the cycles
    self.bus.scheduler.tick(2);

    previous_cycles
  }

  pub fn load_8(&mut self, address: u32) -> (u8, u16) {
    let previous_cycles = self.synchronize_and_get_current_cycles();

    let result = self.bus.mem_read_8(address);

    let duration = (self.bus.scheduler.cycles - previous_cycles) as u16;

    (result, duration)
  }

  /**
   * TODO: This currently doesn't do anything, but
   * in the future I may refactor the code
   * to use it properly.
   */
  fn synchronize_load(&mut self) {
    self.free_cycles[self.free_cycles_reg] = 0;
  }

  pub fn tick_instruction(&mut self) {
    self.bus.scheduler.tick(1);
  }
}