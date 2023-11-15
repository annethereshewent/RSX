use std::{rc::Rc, cell::Cell, collections::VecDeque};

use crate::{cpu::interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt}, spu::SPU};

#[derive(PartialEq)]
pub enum ControllerMode {
  Idle,
  ParamTransfer,
  CommandTransfer,
  CommandExecute,
  ResponseClear,
  ResponseTransfer,
  InterruptTransfer
}

#[derive(PartialEq)]
pub enum DriveMode {
  Idle,
  Seek,
  Read,
  Play,
  GetStat
}

pub struct Cdrom {
  interrupts: Rc<Cell<InterruptRegisters>>,
  index: u8,
  interrupt_enable: u8,
  interrupt_flags: u8,
  param_buffer: VecDeque<u8>,
  response_buffer: VecDeque<u8>,
  controller_response_buffer: VecDeque<u8>,
  command: Option<u8>,
  current_command: u8,
  cycles: i32,
  controller_cycles: i32,
  controller_mode: ControllerMode,
  controller_param_buffer: VecDeque<u8>,
  controller_interrupt_flags: u8
}

impl Cdrom {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    Self {
      interrupts,
      index: 0,
      interrupt_enable: 0,
      interrupt_flags: 0,
      param_buffer: VecDeque::with_capacity(16),
      response_buffer: VecDeque::with_capacity(16),
      controller_param_buffer: VecDeque::with_capacity(16),
      controller_response_buffer: VecDeque::with_capacity(16),
      command: None,
      current_command: 0,
      cycles: 0,
      controller_cycles: 0,
      controller_mode: ControllerMode::Idle,
      controller_interrupt_flags: 0
    }
  }

  pub fn tick_counter(&mut self, cycles: i32, spu: &mut SPU) {
    self.cycles += cycles;

    while self.cycles >= 768 {
      self.cycles -= 768;

      self.tick(cycles, spu);
    }
  }

  pub fn tick(&mut self, cycles: i32, spu: &mut SPU) {
    self.tick_controller(cycles);
    self.tick_drive(cycles, spu);

    if (self.interrupt_enable & self.interrupt_flags & 0x1f) != 0 {
      let mut interrupts = self.interrupts.get();
      interrupts.status.set_interrupt(Interrupt::Cdrom);
      self.interrupts.set(interrupts);
    }
  }

  pub fn tick_drive(&mut self, cycles: i32, spu: &mut SPU) {

  }

  pub fn tick_controller(&mut self, cycles: i32) {
    self.controller_cycles -= cycles;

    if self.controller_cycles < 0 {
      match self.controller_mode {
        ControllerMode::Idle => {
          if self.command.is_some() {
            if !self.param_buffer.is_empty() {
              self.controller_mode = ControllerMode::ParamTransfer;
            } else {
              self.controller_mode = ControllerMode::CommandTransfer;
            }

            self.controller_cycles += cycles;
          }
        },
        ControllerMode::ParamTransfer => {
          if !self.param_buffer.is_empty() {
            let param = self.param_buffer.pop_front().unwrap();

            self.controller_param_buffer.push_back(param);
          } else {
            self.controller_mode = ControllerMode::CommandTransfer;
          }

          self.controller_cycles += 10;
        }
        ControllerMode::CommandTransfer => {
          self.current_command = self.command.take().unwrap();

          self.controller_mode = ControllerMode::CommandExecute;

          self.controller_cycles += 10;
        }
        ControllerMode::CommandExecute => {
          let command = self.current_command;

          self.controller_cycles += 10;

          self.controller_response_buffer.clear();

          self.execute(command);

          self.controller_param_buffer.clear();

          self.controller_mode = ControllerMode::ResponseClear;
        }
        ControllerMode::ResponseClear => {
          if !self.response_buffer.is_empty() {
            self.response_buffer.pop_front();
          } else {
            self.controller_mode = ControllerMode::ResponseTransfer;
          }

          self.controller_cycles += 10;
        }
        ControllerMode::ResponseTransfer => {
          if !self.controller_response_buffer.is_empty() {
            self.response_buffer.push_back(self.controller_response_buffer.pop_front().unwrap());
          } else {
            self.controller_mode = ControllerMode::InterruptTransfer
          }

          self.controller_cycles += 10;
        }
        ControllerMode::InterruptTransfer => {
          if self.interrupt_flags == 0 {
            self.interrupt_flags = self.controller_interrupt_flags;

            self.controller_mode = ControllerMode::Idle;
            self.controller_cycles += 10;
          } else {
            self.controller_cycles += 1;
          }
        }
        _ => todo!("controller mode still not implemented")
      }
    }
  }

  pub fn execute(&mut self, command: u8) {
    let mut interrupt = 0x3;
    match command {
      0x19 => {
        let sub_function = self.controller_param_buffer.pop_front().unwrap();
        // this gets the CD-ROM BIOS version and date. These values should suffice for now.
        if sub_function == 0x20 {
          self.controller_response_buffer.push_back(0x97);
          self.controller_response_buffer.push_back(0x01);
          self.controller_response_buffer.push_back(0x10);
          self.controller_response_buffer.push_back(0xc2);
        } else {
          panic!("unsupported subfunction given: {:x}", sub_function);
        }
      }
      _ => todo!("command not implemented yet: {:x}", command)
    }

    self.controller_interrupt_flags = interrupt;
  }

  pub fn read(&mut self, address: u32) -> u8 {
    match address & 0x3 {
      0 => {
        let mut value = self.index & 0x3;

        value |= ((self.controller_mode != ControllerMode::Idle) as u8) << 7;
        value |= (!self.response_buffer.is_empty() as u8) << 5;
        value |= ((self.param_buffer.len() < 16) as u8) << 4;
        value |= (self.param_buffer.is_empty() as u8) << 3;

        value
      },
      1 => self.response_buffer.pop_front().unwrap(),
      3 => {
        match self.index {
          0 => (0b111 << 5) | self.interrupt_enable,
          1 => (0b111 << 5) | self.interrupt_flags,
          _ => todo!("offset 3 with index {} not implemented", self.index)
        }
      }
      _ => todo!("not implemented yet: {} (index = {})", address & 0x3, self.index)
    }
  }

  pub fn write(&mut self, address: u32, value: u8) {
    match address & 0x3 {
      0 => self.index = value & 0x3,
      1 => {
        match self.index {
          0 => self.command = Some(value),
          _ => panic!("offset 1 with index {} not implemented", self.index)
        }
      }
      2 => {
        match self.index {
          0 => self.param_buffer.push_back(value),
          1 => self.interrupt_enable = value & 0x1f,
          _ => panic!("offset 2 with index {} not implemented yet", {self.index})
        }
      }
      3 => {
        match self.index {
          1 => {
            // writing 1 to these bits clears them
            self.interrupt_flags &= !(value & 0x1f);

            self.response_buffer.clear();

            if (value >> 6) & 0b1 == 1 {
              self.param_buffer.clear();
            }
          }
          _ => panic!("offset 3 with index {} not implemented yet", self.index)
        }
      }
      _ => todo!("not implemented yet: {:X} with index {}", address, self.index)
    }
  }
}