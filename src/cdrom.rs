use std::{rc::Rc, cell::Cell, collections::VecDeque};

use crate::{cpu::interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt}, spu::SPU};


#[derive(PartialEq)]
pub enum SubResponse {
  Disabled,
  GetID,
  GetStat
}

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
  drive_cycles: i32,
  controller_mode: ControllerMode,
  controller_param_buffer: VecDeque<u8>,
  controller_interrupt_flags: u8,
  subresponse: SubResponse,
  subresponse_cycles: i32,
  ss: u8,
  mm: u8,
  sect: u8,
  drive_mode: DriveMode,
  next_drive_mode: DriveMode
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
      subresponse_cycles: 0,
      drive_cycles: 0,
      controller_mode: ControllerMode::Idle,
      subresponse: SubResponse::Disabled,
      drive_mode: DriveMode::Idle,
      next_drive_mode: DriveMode::Idle,
      controller_interrupt_flags: 0,
      ss: 0,
      mm: 0,
      sect: 0
    }
  }

  pub fn tick_counter(&mut self, cycles: i32, spu: &mut SPU) {
    self.cycles += cycles;

    while self.cycles >= 768 {
      self.cycles -= 768;

      self.tick(spu);
    }
  }

  fn tick(&mut self, spu: &mut SPU) {
    self.tick_subresponse();
    self.tick_drive(spu);
    self.tick_controller();

    if (self.interrupt_enable & self.interrupt_flags & 0x1f) != 0 {
      let mut interrupts = self.interrupts.get();
      interrupts.status.set_interrupt(Interrupt::Cdrom);
      self.interrupts.set(interrupts);
    }
  }

  fn tick_subresponse(&mut self) {
    self.subresponse_cycles -= 1;

    if self.subresponse_cycles <= 0 {
      match self.subresponse {
        SubResponse::Disabled => self.subresponse_cycles += 1,
        SubResponse::GetID => {
          // per https://psx-spx.consoledev.net/cdromdrive/#getid-command-1ah-int3stat-int25-statflagstypeatipscex
          /*
            1st byte: stat  (as usually, but with bit3 same as bit7 in 2nd byte)
            2nd byte: flags (bit7=denied, bit4=audio... or reportedly import, uh?)
              bit7: Licensed (0=Licensed Data CD, 1=Denied Data CD or Audio CD)
              bit6: Missing  (0=Disk Present, 1=Disk Missing)
              bit4: Audio CD (0=Data CD, 1=Audio CD) (always 0 when Modchip installed)
            3rd byte: Disk type (from TOC Point=A0h) (eg. 00h=Audio or Mode1, 20h=Mode2)
            4th byte: Usually 00h (or 8bit ATIP from Point=C0h, if session info exists)
              that 8bit ATIP value is taken form the middle 8bit of the 24bit ATIP value
            5th-8th byte: SCEx region (eg. ASCII "SCEE" = Europe) (0,0,0,0 = Unlicensed)
           */

          if self.interrupt_flags == 0 {
            self.controller_response_buffer.push_back(0x2);
            self.controller_response_buffer.push_back(0x0);
            self.controller_response_buffer.push_back(0x20);
            self.controller_response_buffer.push_back(0x0);
            self.controller_response_buffer.push_back('S' as u8);
            self.controller_response_buffer.push_back('C' as u8);
            self.controller_response_buffer.push_back('E' as u8);
            self.controller_response_buffer.push_back('A' as u8);

            self.controller_mode = ControllerMode::ResponseClear;
            self.controller_interrupt_flags = 0x2;

            self.controller_cycles += 10;

            self.subresponse = SubResponse::Disabled;
          }

          self.subresponse_cycles += 1;
        }
        SubResponse::GetStat => {
          // this is supposed to get the table of contents, but apparently we can effectively just get stat again as the second byte and issue an interrupt
          if self.interrupt_flags == 0 {
            self.push_stat();

            self.controller_mode = ControllerMode::ResponseClear;

            self.controller_interrupt_flags = 0x2;

            self.controller_cycles += 10;

            self.subresponse = SubResponse::Disabled;
          }

          self.subresponse_cycles += 1;
        }
      }
    }
  }

  fn tick_drive(&mut self, spu: &mut SPU) {

  }

  fn tick_controller(&mut self) {
    self.controller_cycles -= 1;

    if self.controller_cycles <= 0 {
      match self.controller_mode {
        ControllerMode::Idle => {
          if self.command.is_some() {
            if !self.param_buffer.is_empty() {
              self.controller_mode = ControllerMode::ParamTransfer;
            } else {
              self.controller_mode = ControllerMode::CommandTransfer;
            }

            self.controller_cycles += 1;
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
      }
    }
  }

  pub fn execute(&mut self, command: u8) {
    let mut interrupt = 0x3;
    match command {
      0x01 => self.push_stat(),
      0x02 => self.setloc(),
      // 0x15 | 0x16 => self.seek(),
      0x19 => {
        let sub_function = self.controller_param_buffer.pop_front().unwrap();
        // per https://psx-spx.consoledev.net/cdromdrive/#19h20h-int3yymmddver
        // 97h,01h,10h,C2h  ;PSX (PU-18) (us/eur)     10 Jan 1997, version vC2 (a)
        if sub_function == 0x20 {
          self.controller_response_buffer.push_back(0x97);
          self.controller_response_buffer.push_back(0x01);
          self.controller_response_buffer.push_back(0x10);
          self.controller_response_buffer.push_back(0xc2);
        } else {
          panic!("unsupported subfunction given: {:x}", sub_function);
        }
      }
      0x1a => {
        self.push_stat();

        self.subresponse = SubResponse::GetID;

        self.subresponse_cycles += 50;
      }
      0x1e => {
        self.push_stat();

        self.subresponse = SubResponse::GetStat;
        self.subresponse_cycles += 44100;
      }
      _ => todo!("command not implemented yet: {:x}", command)
    }

    self.controller_interrupt_flags = interrupt;
  }

  fn push_stat(&mut self) {
    let stat = self.get_stat();
    self.controller_response_buffer.push_back(stat);
  }

  fn setloc(&mut self) {
    self.push_stat();

    self.mm = self.controller_param_buffer.pop_front().unwrap();
    self.ss = self.controller_param_buffer.pop_front().unwrap();
    self.sect = self.controller_param_buffer.pop_front().unwrap();
  }

  fn seek(&mut self) {
    self.push_stat();

    self.drive_mode = DriveMode::Seek;

  }

  fn get_stat(&self) -> u8 {
    // bit 1 is for the "motor on" status, should always be 1 in our case
    let mut val = 0b10;
    val |= ((self.drive_mode == DriveMode::Play) as u8) << 7;
    val |= ((self.drive_mode == DriveMode::Seek) as u8) << 6;
    val |= ((self.drive_mode == DriveMode::Read) as u8) << 5;

    val
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
      1 => if self.response_buffer.is_empty() { 0 } else { self.response_buffer.pop_front().unwrap() },
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