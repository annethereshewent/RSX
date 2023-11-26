use std::{collections::VecDeque, mem};

const ZIGZAG_TABLE: [usize; 64] = [
  0,  1,  5,  6,  14, 15, 27, 28,
  2,  4,  7,  13, 16, 26, 29, 42,
  3,  8,  12, 17, 25, 30, 41, 43,
  9,  11, 18, 24, 31, 40, 44, 53,
  10, 19, 23, 32, 39, 45, 52, 54,
  20, 22, 33, 38, 46, 51, 55, 60,
  21, 34, 37, 47, 50, 56, 59, 61,
  35, 36, 48, 49, 57, 58, 62, 63
];


#[derive(PartialEq, Clone, Copy)]
pub enum OutputDepth {
  FourBit = 0,
  EightBit = 1,
  TwentyfourBit = 2,
  FifteenBit = 3
}

#[derive(Copy, Clone)]
pub enum BlockType {
  Cr = 0,
  Cb = 1,
  Yb = 2
}

pub enum Qt {
  Uv = 0,
  Y = 1
}

#[derive(Copy, Clone)]
struct Block {
  pub data: [i16; 64]
}

impl Block {
  pub fn new() -> Self {
    Self {
      data: [0; 64]
    }
  }
}

pub struct Mdec {
  data_out: VecDeque<u8>,
  data_in: VecDeque<u16>,
  data_output_depth: OutputDepth,
  data_bit15: bool,
  data_output_signed: bool,
  dma1_enabled: bool,
  dma0_enabled: bool,
  words_remaining: u16,
  blocks: [Block; 3],
  current_block: usize,
  processing: bool,
  command: u32,
  luminance_and_color: bool,
  luminance_quant_table: [u8; 64],
  color_quant_table: [u8; 64],
  zagzig_table: [usize; 64],
  scale_table: [i16; 64],
  output: [u8; 768]
}

impl Mdec {
  pub fn new() -> Self {
    let mut zagzig_table = [0; 64];

    for i in 0..64 {
      zagzig_table[ZIGZAG_TABLE[i]] = i;
    }

    Self {
      data_bit15: false,
      data_in: VecDeque::new(),
      data_out: VecDeque::new(),
      dma0_enabled: false,
      dma1_enabled: false,
      data_output_depth: OutputDepth::FourBit,
      data_output_signed: false,
      words_remaining: 0,
      current_block: 0,
      processing: false,
      command: 0,
      luminance_and_color: false,
      luminance_quant_table: [0; 64],
      color_quant_table: [0; 64],
      blocks: [Block::new(); 3],
      zagzig_table,
      scale_table: [0; 64],
      output: [0; 768]
    }
  }

  pub fn read_status(&self) -> u32 {
    let mut status = (self.words_remaining - 1) as u32;

    status |= (self.data_bit15 as u32) << 23;
    status |= (self.data_output_signed as u32) << 24;
    status |= (self.data_output_depth as u32) << 25;
    status |= (self.dma1_enabled as u32) << 27;
    status |= (self.dma0_enabled as u32) << 28;
    status |= (self.processing as u32) << 29;
    status |= (!(self.data_in.is_empty()) as u32) << 30;
    status |= (self.data_out.is_empty() as u32) << 31;

    status

  }

  pub fn read_dma(&mut self) -> u32 {
    let byte0 = self.data_out.pop_front().unwrap() as u32;
    let byte1 = self.data_out.pop_front().unwrap() as u32;
    let byte2 = self.data_out.pop_front().unwrap() as u32;
    let byte3 = self.data_out.pop_front().unwrap() as u32;

    byte0 | (byte1 << 8) | (byte2 << 16) | (byte3 << 24)
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
        self.data_output_depth = match (value >> 27) & 0b11 {
          0 => OutputDepth::FourBit,
          1 => OutputDepth::EightBit,
          2 => OutputDepth::TwentyfourBit,
          3 => OutputDepth::FifteenBit,
          _ => unreachable!()
        };
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
        1 => self.decode_macroblocks(),
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
        }
        3 => {
          for i in 0..64 {
            let half_word = self.data_in.pop_front().unwrap();
            self.scale_table[i] = half_word as i16;
          }
        }
        _ => panic!("invalid command received: {}", self.command)
      }

      self.processing = false;
    }

  }

  fn yuv_to_rgb(&mut self, xx: usize, yy: usize) {
    for y in 0..8 {
      for x in 0..8 {
        let mut r = self.blocks[BlockType::Cr as usize].data[((x + xx) / 2) + ((y + yy) / 2) * 8];
        let mut b = self.blocks[BlockType::Cb as usize].data[((x + xx) / 2) + ((y + yy) / 2) * 8];
        let mut g = (-0.3437 * (b as f32) + (-0.7143 * r as f32)) as i16;

        r = (1.402 * r as f32) as i16;
        b = (1.772 * b as f32) as i16;

        let l = self.blocks[BlockType::Yb as usize].data[x + y * 8];

        r = Mdec::min_max(r + l);
        g = Mdec::min_max(g + l);
        b = Mdec::min_max(b + l);

        if !self.data_output_signed {
          r ^= 0x80;
          g ^= 0x80;
          b ^= 0x80;
        }

        if self.data_output_depth == OutputDepth::FifteenBit {
          let offset = ((x + xx) + (y + yy) * 16) * 2;

          let r5bit = ((r as u8) >> 3) as u16;
          let g5bit = ((g as u8) >> 3) as u16;
          let b5bit = ((b as u8) >> 3) as u16;

          let mut data = r5bit | (g5bit << 5) | ( b5bit << 10);

          if self.data_bit15 {
            data |= 1 << 15;
          }

          self.output[offset] = data as u8;
          self.output[offset + 1] = (data >> 8) as u8;
        } else if self.data_output_depth == OutputDepth::TwentyfourBit {
          let offset = ((x + xx) + (y + yy) * 16) * 3;

          self.output[offset] = r as u8;
          self.output[offset + 1] = g as u8;
          self.output[offset + 2] = b as u8;
        }
        // TODO: support the other output depths if needed
      }
    }
  }

  fn min_max(val: i16) -> i16 {
    if val < -128 {
      return -128;
    } else if val > 127 {
      return 127;
    }

    val
  }

  fn decode_macroblocks(&mut self) {
    // see https://psx-spx.consoledev.net/macroblockdecodermdec/#mdec-decompression

    self.output = [0; 768];

    while self.data_in.len() > 0 {
      let processed = match self.current_block {
        0 => self.decode_block(BlockType::Cr, Qt::Uv),
        1 => self.decode_block(BlockType::Cb, Qt::Uv),
        2 => {
          let processed = self.decode_block(BlockType::Yb, Qt::Y);
          self.yuv_to_rgb(0, 0);
          processed
        }
        3 => {
         let processed = self.decode_block(BlockType::Yb, Qt::Y);
          self.yuv_to_rgb(8, 0);
          processed
        }
        4 => {
          let processed = self.decode_block(BlockType::Yb, Qt::Y);
          self.yuv_to_rgb(0, 8);
          processed
        }
        5 => {
          let processed = self.decode_block(BlockType::Yb, Qt::Y);
          self.yuv_to_rgb(8, 8);

          if self.data_output_depth == OutputDepth::FifteenBit {
            for i in 0..512 {
              self.data_out.push_back(self.output[i]);
            }
          } else if self.data_output_depth == OutputDepth::TwentyfourBit {
            for i in 0..768 {
              self.data_out.push_back(self.output[i]);
            }
          }

          processed
        }

        _ => unreachable!()
      };

      if processed {
        self.current_block += 1;

        if self.current_block == 6 {
          self.current_block = 0;
        }
      }
    }
  }

  fn decode_block(&mut self, block_type: BlockType, qt: Qt) -> bool {
    let block = &mut self.blocks[block_type as usize];
    for i in 0..64 {
      block.data[i] = 0;
    }

    let mut data = self.data_in.pop_front().unwrap();

    while data == 0xfe00 {
      if self.data_in.is_empty() {
        return false;
      }

      data = self.data_in.pop_front().unwrap();
    }

    let mut k = 0;

    let quant_table = match qt {
      Qt::Uv => self.color_quant_table,
      Qt::Y => self.luminance_quant_table
    };

    let q_scale = (data >> 10) & 0x3f;

    let mut dc = Self::sign_extend_i10(data & 0x3ff);

    dc = dc * quant_table[k] as i16;

    while k < 64 {
      if q_scale == 0 {
        dc = (Self::sign_extend_i10(data & 0x3ff)) * 2;
      }

      if dc < -0x400 {
        dc = -0x400;
      } else if dc > 0x3ff {
        dc = 0x3ff;
      }

      if q_scale > 0 {
        block.data[self.zagzig_table[k]] = dc;
      } else if q_scale == 0 {
        block.data[k] = dc;
      }

      if self.data_in.is_empty() {
        return false;
      }

      data = self.data_in.pop_front().unwrap();

      k += (((data >> 10) & 0x3f)+ 1) as usize;

      if k < 64 {
        dc = ((Self::sign_extend_i10(data & 0x3ff)) * quant_table[k] as i16 * q_scale as i16 + 4) / 8;
      }
    }

    self.idct_core(block_type);

    true
  }

  fn sign_extend_i10(value: u16) -> i16 {
    ((value << 6) as i16) >> 6
  }

  fn idct_core(&mut self, block_type: BlockType) {
    let block = &mut self.blocks[block_type as usize];

    let temp = &mut [0; 64];
    for _ in 0..2 {
      for x in 0..8 {
        for y in 0..8 {
          let mut sum = 0;
          for z in 0..8 {
            sum += block.data[y + z * 8] as i32 * (self.scale_table[x + z * 8] as i32 / 8);
          }
          temp[x + y * 8] = ((sum + 0xfff) / 0x2000) as i16;
        }
      }
      mem::swap(&mut block.data, temp);
    }
  }

  pub fn write_control(&mut self, value: u32) {
    self.dma0_enabled = (value >> 30) & 0b1 == 1;
    self.dma1_enabled = (value >> 29) & 0b1 == 1;

    if (value >> 31) & 0b1 == 1 {
      //  31    Reset MDEC (0=No change, 1=Abort any command, and set status=80040000h
      self.words_remaining = 0;
      self.processing = false;
      self.current_block = 0;
    }
  }
}
