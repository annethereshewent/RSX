use std::{rc::Rc, cell::Cell};

use crate::{cpu::instruction::Instruction, gpu::{CYCLES_PER_SCANLINE, NUM_SCANLINES_PER_FRAME, GPU_FREQUENCY}};

use self::{bus::Bus, dma::DMA, interrupt::interrupt_registers::InterruptRegisters};

pub mod bus;
pub mod execute;
pub mod instruction;
pub mod dma;
pub mod counter;
pub mod interrupt;
pub mod timers;

// 33.868MHZ
pub const CPU_FREQUENCY: f64 = 33_868_800.0;

pub const CYCLES_PER_FRAME: i64 = ((CYCLES_PER_SCANLINE * NUM_SCANLINES_PER_FRAME) as f64 * (CPU_FREQUENCY / GPU_FREQUENCY)) as i64;

#[derive(Clone, Copy)]
pub enum Cause {
  Interrupt = 0x0,
  LoadAddressError = 0x4,
  StoreAddressError = 0x5,
  IBusError = 0x6,
  DBusError = 0x7,
  SysCall = 0x8,
  Break = 0x9,
  IllegalInstruction = 0xa,
  CoprocessorError = 0xb,
  Overflow = 0xc

}

pub struct COP0 {
  pub sr: u32,
  pub cause: u32,
  pub epc: u32,
  pub jumpdest: u32
}

impl COP0 {
  pub fn bev(&self) -> bool {
    (self.sr >> 22) & 0b1 == 1
  }

  pub fn is_cache_disabled(&self) -> bool {
    self.sr & 0x10000 == 0
  }

  pub fn interrupts_ready(&self) -> bool {
    self.sr & 0b1 == 1 && self.interrupt_mask() != 0
  }

  pub fn interrupt_mask(&self) -> u8 {
    ((self.sr >> 8) as u8) & ((self.cause >> 8) as u8)
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

  pub fn set_interrupt(&mut self, set_active: bool) {
    if set_active {
      self.cause |= 1 << 10;
    } else {
      self.cause &= !(1 << 10);
    }
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
  load: Option<(usize, u32)>,
  dma: Rc<Cell<DMA>>,
  interrupts: Rc<Cell<InterruptRegisters>>,
  current_instruction: u32
}

impl CPU {
  pub fn new(bios: Vec<u8>) -> Self {
    let interrupts = Rc::new(Cell::new(InterruptRegisters::new()));
    let dma = Rc::new(Cell::new(DMA::new()));

    Self {
      pc: 0xbfc0_0000,
      next_pc: 0xbfc0_0004,
      current_pc: 0xbfc0_0000,
      r: [0; 32],
      hi: 0,
      low: 0,
      bus: Bus::new(bios, interrupts.clone(), dma.clone()),
      load: None,
      branch: false,
      delay_slot: false,
      cop0: COP0 {
        sr: 0,
        cause: 0,
        epc: 0,
        jumpdest: 0
      },
      dma,
      interrupts,
      current_instruction: 0,
    }
  }

  pub fn exception(&mut self, cause: Cause) {
    let exception_address = self.cop0.enter_exception(cause);

    self.cop0.epc = match cause {
      Cause::Interrupt => self.pc,
      _ => self.current_pc
    };

    let coprocessor_exception = if matches!(cause, Cause::Break) {
      0
    } else {
      (self.current_instruction >> 26) & 0b11
    };

    self.cop0.cause |= coprocessor_exception << 28;

    if self.delay_slot {
      self.cop0.epc = self.cop0.epc.wrapping_sub(4);
      self.cop0.cause |= 1 << 31;
      self.cop0.jumpdest = self.pc;
    } else {
      self.cop0.cause &= !(1 << 31);
    }

    self.pc = exception_address;
    self.next_pc = self.pc.wrapping_add(4);
  }

  pub fn check_irqs(&mut self) {
    self.cop0.set_interrupt(self.interrupts.get().pending());
  }

  pub fn step(&mut self) {
    let mut dma = self.dma.get();

    if dma.is_active() {
      if dma.in_gap() {
        dma.tick_gap();
        self.dma.set(dma);

        if dma.chopping_enabled() {
          return;
        }
      } else {
        let count = dma.tick(&mut self.bus);
        self.dma.set(dma);

        self.bus.tick(count);

        return;
      }
    }

    self.current_pc = self.pc;

    self.check_irqs();

    let instr = self.fetch_instruction();
    self.current_instruction = instr;

    self.delay_slot = self.branch;
    self.branch = false;

    if self.current_pc & 0b11 != 0 {
      self.exception(Cause::LoadAddressError);

      self.execute_load_delay();

      return;
    }

    // check if we need to handle an interrupt by checking cop0 status register and interrupt mask bits in cause and sr
    if self.cop0.interrupts_ready() {
      self.exception(Cause::Interrupt);

      self.execute_load_delay();

      return;
    }

    // println!("executing instruction {:032b} at address {:08x}", instr, self.current_pc);

    self.pc = self.next_pc;
    self.next_pc = self.next_pc.wrapping_add(4);

    self.tick_instruction();

    self.execute(Instruction::new(instr));

    self.bus.reset_cycles();
  }

  pub fn set_reg(&mut self, rt: usize, val: u32) {
    if rt != 0 {
      self.r[rt] = val;
    }
  }

  pub fn fetch_instruction(&mut self) -> u32 {
    self.bus.tick(5);
    // TODO: add caching code later

    self.bus.mem_read_32(self.pc)
  }

  pub fn store_32(&mut self, address: u32, value: u32) {
    self.bus.tick(5);

    self.bus.mem_write_32(address, value);
  }

  pub fn store_16(&mut self, address: u32, value: u16) {
    self.bus.tick(5);

    self.bus.mem_write_16(address, value)
  }

  pub fn store_8(&mut self, address: u32, value: u8) {
    self.bus.tick(5);

    self.bus.mem_write_8(address, value)
  }

  // TODO: refactor this into just one method
  pub fn load_32(&mut self, address: u32) -> u32 {
    self.bus.tick(5);

    self.bus.mem_read_32(address)
  }

  pub fn load_16(&mut self, address: u32) -> u16 {
    self.bus.tick(5);

    self.bus.mem_read_16(address)
  }

  pub fn load_8(&mut self, address: u32) -> u8 {
    self.bus.tick(5);

    self.bus.mem_read_8(address)
  }

  pub fn tick_instruction(&mut self) {
    self.bus.tick(1);
  }
}