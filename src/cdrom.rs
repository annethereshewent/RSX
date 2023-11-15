use std::{rc::Rc, cell::Cell, collections::VecDeque};

use crate::{cpu::interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt}, spu::SPU};

const CDROM_CYCLES: i32 = 768;

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

#[derive(PartialEq, Clone, Copy)]
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
  next_drive_mode: DriveMode,
  double_speed: bool,
  processing_seek: bool,
  send_adpcm_sectors: bool,
  report_interrupts: bool,
  xa_filter: bool,
  sector_size: bool
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
      sect: 0,
      double_speed: false,
      processing_seek: false,
      send_adpcm_sectors: false,
      report_interrupts: false,
      xa_filter: false,
      sector_size: false
    }
  }

  pub fn tick_counter(&mut self, cycles: i32, spu: &mut SPU) {
    self.cycles += cycles;

    if self.cycles >= CDROM_CYCLES {
      let cd_cycles = self.cycles / CDROM_CYCLES;
      self.cycles %= CDROM_CYCLES;

      self.tick(cd_cycles, spu);
    }
  }

  fn tick(&mut self, cycles: i32, spu: &mut SPU) {
    self.tick_subresponse(cycles);
    self.tick_drive(cycles, spu);
    self.tick_controller(cycles);

    if (self.interrupt_enable & self.interrupt_flags & 0x1f) != 0 {
      let mut interrupts = self.interrupts.get();
      interrupts.status.set_interrupt(Interrupt::Cdrom);
      self.interrupts.set(interrupts);
    }
  }

  pub fn subresponse_get_id(&mut self) {
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

  fn subresponse_get_stat(&mut self) {
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

  fn tick_subresponse(&mut self, cycles: i32) {
    self.subresponse_cycles -= cycles;

    if self.subresponse_cycles <= 0 {
      match self.subresponse {
        SubResponse::Disabled => self.subresponse_cycles += cycles,
        SubResponse::GetID => self.subresponse_get_id(),
        SubResponse::GetStat => self.subresponse_get_stat()
      }
    }
  }

  fn seek_drive(&mut self) {
    self.processing_seek = false;

    match self.next_drive_mode {
      DriveMode::Read | DriveMode::Play => {
        let divisor = if self.double_speed { 150 } else { 75 };

        self.drive_cycles += 44100 / divisor;
      }
      _ => self.drive_cycles += 10
    }

    self.drive_mode = self.next_drive_mode;
  }

  fn play_drive(&mut self) {
    todo!("play_drive not implemented");
  }

  fn read_drive(&mut self) {
    todo!("read_drive not implemented");
  }

  fn drive_get_stat(&mut self) {
    if self.interrupt_flags == 0 {
      self.push_stat();

      self.controller_interrupt_flags = 0x2;

      self.controller_mode = ControllerMode::ResponseClear;
      self.controller_cycles += 10;

      self.drive_mode = DriveMode::Idle;
    }

    self.drive_cycles += 1;
  }

  fn tick_drive(&mut self, cycles: i32, spu: &mut SPU) {
    self.drive_cycles -= cycles;

    if self.drive_cycles <= 0 {
      match self.drive_mode {
        DriveMode::Idle => self.drive_cycles += cycles,
        DriveMode::Seek => self.seek_drive(),
        DriveMode::Play => self.play_drive(),
        DriveMode::Read => self.read_drive(),
        DriveMode::GetStat => self.drive_get_stat()
      }
    }
  }

  fn controller_check_commands(&mut self, cycles: i32) {
    if self.command.is_some() {
      if !self.param_buffer.is_empty() {
        self.controller_mode = ControllerMode::ParamTransfer;
      } else {
        self.controller_mode = ControllerMode::CommandTransfer;
      }

      self.controller_cycles += cycles;
    }
  }

  fn controller_param_transfer(&mut self) {
    if !self.param_buffer.is_empty() {
      let param = self.param_buffer.pop_front().unwrap();

      self.controller_param_buffer.push_back(param);
    } else {
      self.controller_mode = ControllerMode::CommandTransfer;
    }

    self.controller_cycles += 10;
  }

  fn controller_command_transfer(&mut self) {
    self.current_command = self.command.take().unwrap();

    self.controller_mode = ControllerMode::CommandExecute;

    self.controller_cycles += 10;
  }

  fn controller_command_execute(&mut self) {
    let command = self.current_command;

    self.controller_cycles += 10;

    self.controller_response_buffer.clear();

    self.execute(command);

    self.controller_param_buffer.clear();

    self.controller_mode = ControllerMode::ResponseClear;
  }

  fn controller_response_clear(&mut self) {
    if !self.response_buffer.is_empty() {
      self.response_buffer.pop_front();
    } else {
      self.controller_mode = ControllerMode::ResponseTransfer;
    }

    self.controller_cycles += 10;
  }

  fn controller_response_transfer(&mut self) {
    if !self.controller_response_buffer.is_empty() {
      self.response_buffer.push_back(self.controller_response_buffer.pop_front().unwrap());
    } else {
      self.controller_mode = ControllerMode::InterruptTransfer
    }

    self.controller_cycles += 10;
  }

  fn controller_interrupt_transfer(&mut self) {
    if self.interrupt_flags == 0 {
      self.interrupt_flags = self.controller_interrupt_flags;

      self.controller_mode = ControllerMode::Idle;
      self.controller_cycles += 10;
    } else {
      self.controller_cycles += 1;
    }
  }

  fn tick_controller(&mut self, cycles: i32) {
    self.controller_cycles -= cycles;

    if self.controller_cycles <= 0 {
      match self.controller_mode {
        ControllerMode::Idle => self.controller_check_commands(cycles),
        ControllerMode::ParamTransfer => self.controller_param_transfer(),
        ControllerMode::CommandTransfer => self.controller_command_transfer(),
        ControllerMode::CommandExecute => self.controller_command_execute(),
        ControllerMode::ResponseClear => self.controller_response_clear(),
        ControllerMode::ResponseTransfer => self.controller_response_transfer(),
        ControllerMode::InterruptTransfer => self.controller_interrupt_transfer()
      }
    }
  }

  fn execute(&mut self, command: u8) {
    let mut interrupt = 0x3;
    match command {
      0x01 => self.push_stat(),
      0x02 => self.setloc(),
      0x06 => self.readn(),
      0x0e => self.setmode(),
      0x15 | 0x16 => self.seek(),
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

  fn setmode(&mut self) {
    let param = self.controller_param_buffer.pop_front().unwrap();

    self.double_speed = (param >> 7) & 0b1 == 1;
    self.send_adpcm_sectors = (param >> 6) & 0b1 == 1;
    self.sector_size = (param >> 5) & 0b1 == 1;
    // bit 4 is the ignore bit, but according to no$psx its purpose is unknown.
    // ignoring it for now

    self.xa_filter = (param >> 3) & 0b1 == 1;
    self.report_interrupts = (param >> 2) & 0b1 == 1;

  }

  fn readn(&mut self) {
    if self.processing_seek {
      self.drive_mode = DriveMode::Seek;
      self.next_drive_mode = DriveMode::Read;

      self.drive_cycles += if self.double_speed {
        140
      } else {
        280
      };
    } else {
      self.drive_mode = DriveMode::Read;

      let divisor = if self.double_speed {
        150
      } else {
        75
      };

      self.drive_cycles += 44100 / divisor;
    }

    self.push_stat();
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

    self.processing_seek = true;
  }

  fn seek(&mut self) {
    self.push_stat();

    self.drive_mode = DriveMode::Seek;
    self.next_drive_mode = DriveMode::GetStat;

    self.drive_cycles += if self.double_speed {
      28
    } else {
      14
    };
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