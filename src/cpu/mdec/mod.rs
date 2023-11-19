use std::collections::VecDeque;

pub struct Mdec {
  data_out: VecDeque<u8>,
  data_in: VecDeque<u16>,
  data_output_depth: u32,
  data_bit15: bool,
  data_output_signed: bool,
  dma1_enabled: bool,
  dma0_enabled: bool,
  words_remaining: u16,
  current_block: usize,
  processing: bool,
  command: u32,
  luminance_and_color: bool,
  luminance_quant_table: [u8; 64],
  color_quant_table: [u8; 64]
}

impl Mdec {
  pub fn new() -> Self {
    Self {
      data_bit15: false,
      data_in: VecDeque::new(),
      data_out: VecDeque::new(),
      dma0_enabled: false,
      dma1_enabled: false,
      data_output_depth: 0,
      data_output_signed: false,
      words_remaining: 0,
      current_block: 0,
      processing: false,
      command: 0,
      luminance_and_color: false,
      luminance_quant_table: [0; 64],
      color_quant_table: [0; 64]
    }
  }

  pub fn read_status(&self) -> u32 {
    let mut status = (self.words_remaining - 1) as u32;

    status |= (self.data_bit15 as u32) << 23;
    status |= (self.data_output_signed as u32) << 24;
    status |= self.data_output_depth << 25;
    status |= (self.dma1_enabled as u32) << 27;
    status |= (self.dma0_enabled as u32) << 28;
    status |= (self.processing as u32) << 29;
    status |= (!(self.data_in.is_empty()) as u32) << 30;
    status |= (self.data_out.is_empty() as u32) << 31;

    status

  }

  pub fn write_command(&mut self, value: u32) {
    if self.processing {
      self.process_command(value);

      return;
    }

    self.command = value >> 29;

    self.processing = true;

    match self.command {
      1 => {
        self.data_output_depth = (value >> 27) & 0b11;
        self.data_output_signed = (value >> 26) & 0b1 == 1;
        self.data_bit15 = (value >> 25) & 0b1 == 1;
        self.words_remaining = (value & 0xffff) as u16;
      }
      2 => {
        // The command word is followed by 64 unsigned parameter bytes for the Luminance Quant Table (used for Y1..Y4),
        // and if Command.Bit0 was set, by another 64 unsigned parameter bytes for the Color Quant Table
        self.luminance_and_color = value & 0b1 == 1;
        self.words_remaining = if self.luminance_and_color {
          32
        } else {
          16
        };
      }
      3 => {
        // The command is followed by 64 signed halfwords with 14bit fractional part
        self.words_remaining = 32;
      }
      _ => panic!("invalid command received: {}", self.command)
    }

  }

  fn process_command(&mut self, value: u32) {
    self.data_in.push_back(value as u16);
    self.data_in.push_back((value >> 16) as u16);

    self.words_remaining -= 1;

    if self.words_remaining == 0 {
      match self.command {
        1 => {

        }
        2 => {
          for i in 0..32 {
            let halfword = self.data_in.pop_front().unwrap();

            self.luminance_quant_table[i * 2] = halfword as u8;
            self.luminance_quant_table[i * 2 + 1] = (halfword >> 8) as u8;
          }

          if self.luminance_and_color {
            for i in 0..32 {
              let halfword = self.data_in.pop_front().unwrap();

              self.color_quant_table[i * 2] = halfword as u8;
              self.color_quant_table[i * 2 + 1] = (halfword >> 8) as u8;
            }
          }

          self.processing = false;
        }
        3 => {

        }
        _ => panic!("invalid command received: {}", self.command)
      }
    }

  }

  pub fn write_control(&mut self, value: u32) {
    self.dma0_enabled = (value >> 30) & 0b1 == 1;
    self.dma1_enabled = (value >> 29) & 0b1 == 1;

    if (value >> 31) & 0b1 == 1 {
      //  31    Reset MDEC (0=No change, 1=Abort any command, and set status=80040000h
      self.words_remaining = 0;
      self.processing = false;
      self.current_block = 4;
    }
  }
}