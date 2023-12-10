use std::{fs::{self, File}, io::{Write, Read, Seek, SeekFrom}, os::unix::fs::FileExt};

#[derive(PartialEq)]
pub enum CardState {
  Idle,
  AwaitingCommand,
  Read,
  Write,
  GetId
}

const MEMORY_CARD_SIZE: usize = 0x20000;

pub struct MemoryCard {
  state: CardState,
  read_step: u8,
  write_step: u8,
  id_step: u8,
  sector_number: u16,
  current_byte: u8,
  checksum: u8,
  checksum_match: bool,
  previous: u8,
  card: Box<[u8]>,
  flag: u8,
  card_file: File
}

impl MemoryCard {
  pub fn new() -> Self {
    let filename = "../cards/memory_card.mcd";

    fs::create_dir_all("../cards").unwrap();

    let file = fs::OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .append(true)
      .open(filename)
      .unwrap();

    Self {
      state: CardState::Idle,
      read_step: 0,
      write_step: 0,
      id_step: 0,
      sector_number: 0,
      current_byte: 0,
      checksum: 0,
      previous: 0,
      checksum_match: false,
      card: vec![0; MEMORY_CARD_SIZE].into_boxed_slice(),
      flag: 0x8,
      card_file: file
    }
  }

  pub fn load_file_contents(&mut self) {
    self.card_file.read_exact(&mut self.card).unwrap();
  }

  pub fn enabled(&self) -> bool {
    self.state != CardState::Idle
  }

  pub fn reset_state(&mut self) {
    self.state = CardState::Idle;
  }

  pub fn reply(&mut self, command: u8) -> u8 {
    let mut reply = 0xff;

    match self.state {
      CardState::Idle => {
        self.state = CardState::AwaitingCommand;
      }
      CardState::AwaitingCommand => {
        reply = self.flag;
        match command {
          0x52 => {
            self.state = CardState::Read;
            self.read_step = 0;
            self.current_byte = 0;
          }
          0x53 => {
            self.state = CardState::GetId;
            self.id_step = 0;
          }
          0x57 => {
            self.state = CardState::Write;
            self.write_step = 0;
            self.current_byte = 0;
          }
          _ => panic!("invalid memory card command received: {:x}", command)
        }
      }
      CardState::Read => reply = self.process_read_command(command),
      CardState::Write => reply = self.process_write_command(command),
      CardState::GetId => reply = self.process_get_id_command(command)
    }

    reply
  }

  fn process_get_id_command(&mut self, _command: u8) -> u8 {
    let mut reply = 0xff;

    let mut should_advance = true;

    match self.id_step {
      0 => reply = 0x5a,
      1 => reply = 0x5d,
      2 => reply = 0x5c,
      3 => reply = 0x5d,
      4 => reply = 0x4,
      5 => reply = 0x0,
      6 => reply = 0x0,
      7 => {
        reply = 0x80;
        should_advance = false;

        self.state = CardState::Idle;
        self.id_step = 0;
      }
      _ => panic!("invalid step: {}", self.id_step)
    }

    if should_advance {
      self.id_step += 1;
    }

    reply
  }

  fn process_write_command(&mut self, command: u8) -> u8 {
    let mut reply = 0xff;
    let mut should_advance = true;

    match self.write_step {
      0 => {
        reply = 0x5a;
        self.flag &= !0x8;
      },
      1 => reply = 0x5d,
      2 => {
        // clear out upper byte then set msb with whatever's in command
        self.sector_number &= 0xff;
        self.sector_number |= (command as u16) << 8;


        reply = 0x0;

        self.previous = command;

        self.checksum = command;
      }
      3 => {
        reply = self.previous;
        self.sector_number &= 0xff00;
        self.sector_number |= command as u16;

        self.checksum ^= command;
      }
      4 => {
        should_advance = false;
        reply = self.previous;

        let sector_address = (self.sector_number * 128) as usize;
        self.write_byte(sector_address + self.current_byte as usize, command);

        self.current_byte += 1;

        self.checksum ^= command;

        if self.current_byte == 128 {
          should_advance = true;
        }
      }
      5 => {
        reply = self.checksum;
        self.checksum_match = command == self.checksum;
      }
      6 => reply = 0x5c,
      7 => reply = 0x5d,
      8 => {
        if self.checksum_match {
          reply = 0x47;
        } else {
          reply = 0x4e;
        }

        self.state = CardState::Idle;
        self.write_step = 0;
      }
      _ => panic!("invalid step: {}", self.write_step)
    }

    if should_advance {
      self.write_step += 1;
    }

    reply
  }

  fn process_read_command(&mut self, command: u8) -> u8 {
    let mut reply = 0xff;
    let mut should_advance = true;

    match self.read_step {
      0 => reply = 0x5a,
      1 => reply = 0x5d,
      2 => {
        // clear out upper byte then set msb with whatever's in command
        self.sector_number &= 0xff;
        self.sector_number |= (command as u16) << 8;

        self.checksum = command;
        self.previous = command;

        reply = 0x0;
      }
      3 => {
        //clear out lower byte then set lsb
        self.sector_number &= 0xff00;
        self.sector_number |= command as u16;

        if self.sector_number > 0x3ff {
          self.sector_number = 0xffff
        }

        self.checksum ^= command;

        reply = self.previous;
      }
      4 => reply = 0x5c,
      5 => reply = 0x5d,
      6 => {
        reply = (self.sector_number >> 8) as u8;
      }
      7 => {
        reply = self.sector_number as u8;
        if self.sector_number > 0x3ff {
          should_advance = false;

          self.state = CardState::Idle;
          self.read_step = 0;
        }
      },
      8 => {
        should_advance = false;
        let sector_address = self.sector_number as usize * 128;
        reply = self.read_byte(sector_address + self.current_byte as usize);

        self.checksum ^= reply;

        self.current_byte += 1;

        if self.current_byte == 128 {
          self.write_to_file();
          should_advance = true;
        }
      }
      9 => {
        reply = self.checksum;
      }
      10 => {
        reply = 0x47;
        should_advance = false;

        self.state = CardState::Idle;
        self.read_step = 0;
      }

      _ => panic!("invalid step given: {}", self.read_step)
    }

    if should_advance {
      self.read_step += 1;
    }

    reply
  }

  fn read_byte(&self, address: usize) -> u8 {
    self.card[address]
  }

  fn write_byte(&mut self, address: usize, byte: u8) {
    self.card[address] = byte;
  }

  fn write_to_file(&mut self) {
    self.card_file.write_all_at(&self.card, 0).unwrap();
    self.card_file.flush().unwrap();

    let mut buffer_copy = [0; MEMORY_CARD_SIZE];

    self.card_file.seek(SeekFrom::Start(0)).unwrap();
    self.card_file.read_exact(&mut buffer_copy).unwrap();

    for i in 0..buffer_copy.len() {
      if buffer_copy[i] != self.card[i] {
        panic!("{} vs {} at index {i}", buffer_copy[i], self.card[i]);
      }
    }
  }
}