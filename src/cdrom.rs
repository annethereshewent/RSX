use std::{rc::Rc, cell::Cell, collections::VecDeque, fs::File, io::{SeekFrom, Read, Seek}};

use crate::{cpu::interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt}, spu::SPU};

const CDROM_CYCLES: i32 = 768;
pub const SECTORS_PER_SECOND: u64 = 75;
pub const SECTORS_PER_MINUTE: u64 = 60 * SECTORS_PER_SECOND;
pub const BYTES_PER_SECTOR: u64 = 2352;
pub const LEAD_IN_SECTORS: u64 = 2 * SECTORS_PER_SECOND;

const HEADER_START: usize = 12;
const SUBHEADER_START: usize = 16;

const DATA_OFFSET: usize = 24;
const ADDR_OFFSET: usize = 12;

#[derive(PartialEq)]
pub enum CdSubheaderMode {
  Video,
  Audio,
  Data,
  Error
}


struct CdHeader {
  mm: u8,
  ss: u8,
  sect: u8,
  mode: u8
}

impl CdHeader {
  pub fn new(buf: &mut [u8]) -> Self {
    let offset = HEADER_START;
    Self {
      mm: Cdrom::bcd_to_u8(buf[offset]),
      ss: Cdrom::bcd_to_u8(buf[offset + 1]),
      sect: Cdrom::bcd_to_u8(buf[offset + 2]),
      mode: buf[offset + 3]
    }
  }
}

#[derive(Copy, Clone)]
struct CdSubheader {
  file: u8,
  channel: u8,
  coding_info: u8,
  sub_mode: u8
}

impl CdSubheader {
  pub fn new(buf: &mut [u8]) -> Self {
    let offset = SUBHEADER_START;
    Self {
      file: buf[offset],
      channel: buf[offset + 1],
      sub_mode: buf[offset + 2],
      coding_info: buf[offset + 3]
    }
  }

  pub fn mode(&self) -> CdSubheaderMode {
    match self.sub_mode & 0xe {
      2 => CdSubheaderMode::Video,
      4 => CdSubheaderMode::Audio,
      0 | 8 => CdSubheaderMode::Data,
      _ => CdSubheaderMode::Error
    }
  }
}

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
  current_ss: u8,
  current_mm: u8,
  current_sect: u8,
  drive_mode: DriveMode,
  next_drive_mode: DriveMode,
  double_speed: bool,
  processing_seek: bool,
  send_adpcm_sectors: bool,
  report_interrupts: bool,
  xa_filter: bool,
  sector_size: bool,
  game_file: File,
  sector_header: CdHeader,
  sector_subheader: CdSubheader,
  sector_buffer: Vec<u8>,

  drive_interrupt_pending: bool,
  pending_stat: u8,

  data_buffer: Vec<u8>,
  data_buffer_pointer: usize,

  is_playing: bool,
  is_seeking: bool,
  is_reading: bool
}

impl Cdrom {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>, game_file: File) -> Self {
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
      current_ss: 0,
      current_mm: 0,
      current_sect: 0,
      double_speed: false,
      processing_seek: false,
      send_adpcm_sectors: false,
      report_interrupts: false,
      xa_filter: false,
      sector_size: false,
      game_file,
      sector_header: CdHeader {
        mm: 0,
        ss: 0,
        sect: 0,
        mode: 0
      },
      sector_subheader: CdSubheader {
        file: 0,
        channel: 0,
        coding_info: 0,
        sub_mode: 0
      },
      sector_buffer: vec![0; 0x930],
      drive_interrupt_pending: false,
      pending_stat: 0,
      data_buffer: vec![0; 0x930],
      data_buffer_pointer: 0,
      is_playing: false,
      is_seeking: false,
      is_reading: false
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

    self.current_mm = self.mm;
    self.current_ss = self.ss;
    self.current_sect = self.sect;

    self.is_playing = false;
    self.is_seeking = false;
    self.is_reading = false;

    match self.next_drive_mode {
      DriveMode::Read => {
        let divisor = if self.double_speed { 150 } else { 75 };

        self.drive_cycles += 44100 / divisor;

        self.is_reading = true;
      }
      DriveMode::Play => {
        let divisor = if self.double_speed { 150 } else { 75 };

        self.drive_cycles += 44100 / divisor;

        self.is_playing = true;
      }
      _ => self.drive_cycles += 10
    }

    self.drive_mode = self.next_drive_mode;
  }

  fn play_drive(&mut self) {
    todo!("play_drive not implemented");
  }

  fn read_drive(&mut self) {
    if !self.is_reading {
      self.drive_mode = DriveMode::Idle;
      self.drive_cycles += 1;

      return;
    }
    self.push_stat();

    let file_pointer = self.get_seek_pointer();

    let mut buf = [0u8; 24];

    self.game_file.seek(SeekFrom::Start(file_pointer)).unwrap();
    self.game_file.read_exact(&mut buf).unwrap();

    let header = CdHeader::new(&mut buf);
    let subheader = CdSubheader::new(&mut buf);

    if header.mm != self.current_mm || header.ss != self.current_ss || header.sect != self.current_sect {
      panic!("mismatched sector info between header and controller");
    }

    if header.mode != 2 {
      panic!("unsupported mode found: {}", header.mode);
    }

    self.sector_header = header;
    self.sector_subheader = subheader;

    self.current_sect += 1;

    if self.current_sect >= 75 {
      self.current_sect = 0;
      self.current_ss += 1;

      if self.current_ss >= 60 {
        self.current_ss = 0;
        self.current_mm += 1;
      }
    }

    match subheader.mode() {
      CdSubheaderMode::Audio => todo!("not implemented yet"),
      CdSubheaderMode::Data => self.read_data(file_pointer),
      CdSubheaderMode::Video => panic!("video not implemented"),
      CdSubheaderMode::Error => panic!("an error occurred parsing subheader")
    }

    let divisor = if self.double_speed { 150 } else { 75 };
    self.drive_cycles += 44100 / divisor;
  }

  fn read_data(&mut self, file_pointer: u64) {
    self.game_file.seek(SeekFrom::Start(file_pointer)).unwrap();
    self.game_file.read_exact(&mut self.sector_buffer).unwrap();

    if self.interrupt_flags == 0 {
      self.interrupt_flags = 1;
      self.response_buffer.push_back(self.get_stat());
    } else {
      self.drive_interrupt_pending = true;
      self.pending_stat = self.get_stat();
    }
  }

  fn get_seek_pointer(&self) -> u64 {
    let mut sector = self.current_ss as u64 * SECTORS_PER_SECOND + self.current_mm as u64 * SECTORS_PER_MINUTE + self.current_sect as u64;

    if sector >= LEAD_IN_SECTORS {
      sector -= LEAD_IN_SECTORS;
    }

    sector * BYTES_PER_SECTOR
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

  pub fn bcd_to_u8(value: u8) -> u8 {
    ((value >> 4) * 10) + (value & 0xf)
  }

  pub fn u8_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
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
      0x09 => self.pause(),
      0x0a => self.init(),
      0x0b | 0x0c => self.push_stat(),
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
      0x1b => self.reads(),
      0x1e => {
        self.push_stat();

        self.subresponse = SubResponse::GetStat;
        self.subresponse_cycles += 44100;
      }
      _ => todo!("command not implemented yet: {:x}", command)
    }

    self.controller_interrupt_flags = interrupt;
  }

  fn init(&mut self) {
    self.push_stat();

    self.double_speed = false;
    self.sector_size = false;

    self.is_playing = false;
    self.is_reading = false;
    self.is_seeking = false;

    self.subresponse = SubResponse::GetStat;
    self.subresponse_cycles += 10;

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

  fn pause(&mut self) {
    self.push_stat();

    if !self.is_playing && !self.is_reading && !self.is_seeking {
      self.subresponse_cycles += 10;
    } else {
      self.subresponse_cycles += if self.double_speed {
        1400
      } else {
        2800
      };
    }

    self.is_playing = false;
    self.is_reading = false;
    self.is_seeking = false;

    self.subresponse = SubResponse::GetStat;
  }

  fn readn(&mut self) {
    self.read_command(true);
  }

  fn reads(&mut self) {
    self.read_command(false);
  }

  fn read_command(&mut self, is_readn: bool) {
    if self.processing_seek {
      self.drive_mode = DriveMode::Seek;
      self.next_drive_mode = DriveMode::Read;

      self.is_seeking = true;
      self.is_reading = false;
      self.is_playing = false;

      self.drive_cycles += if self.double_speed {
        if is_readn { 140 } else { 14 }
      } else {
        if is_readn { 280 } else { 28 }
      };
    } else {
      self.drive_mode = DriveMode::Read;

      self.is_reading = true;
      self.is_playing = false;
      self.is_seeking = false;

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

    self.mm = Self::bcd_to_u8(self.controller_param_buffer.pop_front().unwrap());
    self.ss = Self::bcd_to_u8(self.controller_param_buffer.pop_front().unwrap());
    self.sect = Self::bcd_to_u8(self.controller_param_buffer.pop_front().unwrap());

    self.processing_seek = true;
  }

  fn seek(&mut self) {
    self.push_stat();

    self.drive_mode = DriveMode::Seek;
    self.next_drive_mode = DriveMode::GetStat;

    self.drive_cycles += if self.double_speed {
      14
    } else {
      28
    };

    self.is_seeking = true;
    self.is_playing = false;
    self.is_reading = false;
  }

  fn read_data_buffer(&mut self) -> u8 {
    let offset = if self.sector_size {
      ADDR_OFFSET
    } else {
      DATA_OFFSET
    };

    if self.data_buffer_empty() {
      panic!("data buffer is empty");
    }

    let val = self.data_buffer[offset + self.data_buffer_pointer];

    self.data_buffer_pointer += 1;

    val
  }

  fn data_buffer_empty(&self) -> bool {
    let max = if self.sector_size {
      0x924
    } else {
      0x800
    };

    self.data_buffer_pointer >= max
  }

  fn get_stat(&self) -> u8 {
    // bit 1 is for the "motor on" status, should always be 1 in our case
    let mut val = 0b10;
    val |= (self.is_playing as u8) << 7;
    val |= (self.is_seeking as u8) << 6;
    val |= (self.is_reading as u8) << 5;

    val
  }

  pub fn read_dma(&mut self) -> u32 {
    let byte0 = self.read_data_buffer() as u32;
    let byte1 = self.read_data_buffer() as u32;
    let byte2 = self.read_data_buffer() as u32;
    let byte3 = self.read_data_buffer() as u32;

    byte0 | (byte1 << 8) | (byte2 << 16) | (byte3 << 24)
  }

  pub fn read(&mut self, address: u32) -> u8 {
    match address & 0x3 {
      0 => {
        let mut value = self.index & 0x3;

        value |= ((self.controller_mode != ControllerMode::Idle) as u8) << 7;
        value |= (!self.data_buffer_empty() as u8) << 6;
        value |= (!self.response_buffer.is_empty() as u8) << 5;
        value |= ((self.param_buffer.len() < 16) as u8) << 4;
        value |= (self.param_buffer.is_empty() as u8) << 3;

        value
      },
      1 => if self.response_buffer.is_empty() { 0 } else { self.response_buffer.pop_front().unwrap() },
      2 => self.read_data_buffer(),
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
          3 => (), // cd audio to spu, unimplemented
          _ => panic!("offset 1 with index {} not implemented", self.index)
        }
      }
      2 => {
        match self.index {
          0 => self.param_buffer.push_back(value),
          1 => self.interrupt_enable = value & 0x1f,
          2 => (), // left cd audio to spu, not implemented
          3 => (), // right cd audio to spu, not implemented
          _ => panic!("offset 2 with index {} not implemented yet", {self.index})
        }
      }
      3 => {
        match self.index {
          0 => {
            if (value >> 7) & 0b1 == 0 {
              self.data_buffer_pointer = 0x930;
            } else if self.data_buffer_empty() {
              self.data_buffer_pointer = 0;
              self.data_buffer[..0x930].clone_from_slice(&self.sector_buffer[..0x930]);
            }
          }
          1 => {
            // writing 1 to these bits clears them
            self.interrupt_flags &= !(value & 0x1f);

            if self.interrupt_flags == 0 && self.drive_interrupt_pending {
              self.interrupt_flags = 1;
              self.drive_interrupt_pending = false;
              self.response_buffer.push_back(self.pending_stat);
            }

            self.response_buffer.clear();

            if (value >> 6) & 0b1 == 1 {
              self.param_buffer.clear();
            }
          }
          2 | 3 => (), // more CD audio stuff
          _ => panic!("offset 3 with index {} not implemented yet", self.index)
        }
      }
      _ => todo!("not implemented yet: {:X} with index {}", address, self.index)
    }
  }
}