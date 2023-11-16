use super::instruction::Instruction;

#[derive(Clone, Copy)]
struct Rgb {
  r: u8,
  g: u8,
  b: u8,
  c: u8
}

pub struct COP2 {
  zsf3: i16,
  zsf4: i16,
  h: u16,
  dqa: i16,
  dqb: i32,
  ofx: i32,
  ofy: i32,
  fc: (i32, i32, i32),
  bk: (i32, i32, i32),
  tr: (i32, i32, i32),
  color: [[i16; 3]; 3],
  light: [[i16; 3]; 3],
  rotation: [[i16; 3]; 3],
  v: [(i16, i16, i16); 3],
  rgbc: Rgb,
  otz: u16,
  ir: [i16; 4],
  flags: u32,
  sf: usize,
  mx: usize,
  sv: usize,
  cv: usize,
  lm: bool,
  sxy_fifo: [(i16, i16); 3],
  sz_fifo: [u16; 4],
  rgb_fifo: [Rgb; 3],
  res1: u32,
  mac: [i32; 4],
  lzcs: i32,
  lzcr: i32
}

impl COP2 {
  pub fn new() -> Self {
    Self {
      zsf3: 0,
      zsf4: 0,
      h: 0,
      dqa: 0,
      dqb: 0,
      ofx: 0,
      ofy: 0,
      fc: (0, 0, 0),
      bk: (0, 0, 0),
      color: [[0; 3]; 3],
      light: [[0; 3]; 3],
      rotation: [[0; 3]; 3],
      tr: (0, 0, 0),
      v: [(0,0,0); 3],
      rgbc: Rgb {
        r: 0,
        g: 0,
        b: 0,
        c: 0
      },
      otz: 0,
      ir: [0; 4],
      flags: 0,
      sf: 0,
      mx: 0,
      sv: 0,
      cv: 0,
      lm: false,
      sxy_fifo: [(0, 0); 3],
      sz_fifo: [0; 4],
      rgb_fifo: [Rgb { r: 0, g: 0, b: 0, c: 0 }; 3],
      res1: 0,
      mac: [0; 4],
      lzcs: 0,
      lzcr: 0
    }
  }

  pub fn execute_command(&mut self, instr: Instruction) {
    let command = instr.cop2_command();

    let op_code = command & 0x3f;

    self.sf = if (command >> 19) & 0b1 == 1 {
      12
    } else {
      0
    };

    self.mx = ((command >> 17) & 0x3) as usize;
    self.sv = ((command >> 15) & 0x3) as usize;
    self.cv = ((command >> 13) & 0x3) as usize;

    self.lm = (command >> 10) & 0b1 == 1;

    self.flags = 0;

    match op_code {
      0x06 => self.nclip(),
      0x30 => self.triple_perspective_transform(),
      _ => panic!("unimplemented op code for gte: {:x}", op_code)
    }
  }

  fn nclip(&mut self) {
    println!("clipping completed (just kidding!)");
  }

  fn triple_perspective_transform(&mut self) {
    // per https://psx-spx.consoledev.net/geometrytransformationenginegte/#cop2-0280030h-23-cycles-rtpt-perspective-transformation-triple
    /*
      IR1 = MAC1 = (TRX*1000h + RT11*VX0 + RT12*VY0 + RT13*VZ0) SAR (sf*12)
      IR2 = MAC2 = (TRY*1000h + RT21*VX0 + RT22*VY0 + RT23*VZ0) SAR (sf*12)
      IR3 = MAC3 = (TRZ*1000h + RT31*VX0 + RT32*VY0 + RT33*VZ0) SAR (sf*12)
      SZ3 = MAC3 SAR ((1-sf)*12)                           ;ScreenZ FIFO 0..+FFFFh
      MAC0=(((H*20000h/SZ3)+1)/2)*IR1+OFX, SX2=MAC0/10000h ;ScrX FIFO -400h..+3FFh
      MAC0=(((H*20000h/SZ3)+1)/2)*IR2+OFY, SY2=MAC0/10000h ;ScrY FIFO -400h..+3FFh
      MAC0=(((H*20000h/SZ3)+1)/2)*DQA+DQB, IR0=MAC0/1000h  ;Depth cueing 0..+1000h
    */
    let mac1 = (self.tr.0 * 0x1000 + self.rotation[0][0] as i32 * self.v[0].0 as i32 + self.rotation[0][1] as i32 * self.v[0].1 as i32 + self.rotation[0][2] as i32 * self.v[0].2 as i32)  >> (self.sf * 12);

    println!("triple transform todo!");
  }

  fn push_sx(&mut self, sx: i16) {
    self.sxy_fifo[0].0 = self.sxy_fifo[1].0;
    self.sxy_fifo[1].0 = self.sxy_fifo[2].0;
    self.sxy_fifo[2].0 = sx;
  }

  fn push_sy(&mut self, sy: i16) {
    self.sxy_fifo[0].1 = self.sxy_fifo[1].1;
    self.sxy_fifo[1].1 = self.sxy_fifo[2].1;
    self.sxy_fifo[2].1 = sy;
  }

  pub fn push_sz(&mut self, sz: u16) {
    self.sz_fifo[0] = self.sz_fifo[1];
    self.sz_fifo[1] = self.sz_fifo[2];
    self.sz_fifo[2] = self.sz_fifo[3];
    self.sz_fifo[3] = sz;
  }

  pub fn read_data(&mut self, destination: usize) -> u32 {
    match destination {
      0 => (self.v[0].0 as u16 as u32) | (self.v[0].1 as u16 as u32) << 16,
      1 => self.v[0].2 as u32,
      2 => (self.v[1].0 as u16 as u32) | (self.v[1].1 as u16 as u32) << 16,
      3 => self.v[1].2 as u32,
      4 => (self.v[2].0 as u16 as u32) | (self.v[2].1 as u16 as u32) << 16,
      5 => self.v[1].2 as u32,
      6 => {
        (self.rgbc.r as u32) | (self.rgbc.g as u32) << 8 | (self.rgbc.b as u32) << 16 | (self.rgbc.c as u32) << 24
      }
      7 => self.otz as u32,
      8..=11 => self.ir[destination - 8] as u32,
      12..=14 => (self.sxy_fifo[destination - 12].0 as u32) | (self.sxy_fifo[destination - 12].1 as u32) << 16,
      15 => (self.sxy_fifo[2].0 as u16 as u32) | (self.sxy_fifo[2].1 as u16 as u32) << 16,
      16..=19 => self.sz_fifo[destination - 16] as u16 as u32,
      20..=22 => {
        (self.rgb_fifo[destination - 20].r as u32) | (self.rgb_fifo[destination - 20].g as u32) << 8 | (self.rgb_fifo[destination - 20].b as u32) << 16 | (self.rgb_fifo[destination - 20].c as u32) << 24
      }
      23 => self.res1,
      24..=27 => self.mac[destination - 24] as u32,
      _ => panic!("unsupported destination: {destination}")
    }
  }

  pub fn write_data(&mut self, destination: usize, value: u32) {
    match destination {
      0 => {
        self.v[0].0 = value as i16;
        self.v[0].1 = (value >> 16) as i16;
      }
      1 => self.v[0].2 = value as i16,
      2 => {
        self.v[1].0 = value as i16;
        self.v[1].1 = (value >> 16) as i16;
      }
      3 => self.v[1].2 = value as i16,
      4 => {
        self.v[2].0 = value as i16;
        self.v[2].1 = (value >> 16) as i16;
      }
      5 => self.v[1].2 = value as i16,
      6 => {
        self.rgbc.r = value as u8;
        self.rgbc.g = (value >> 8) as u8;
        self.rgbc.b = (value >> 16) as u8;
        self.rgbc.c = (value >> 24) as u8;
      }
      7 => self.otz = value as u16,
      8..=11 => self.ir[destination - 8] = value as i16,
      12..=14 => {
        self.sxy_fifo[destination - 12].0 = value as i16;
        self.sxy_fifo[destination - 12].1 = (value >> 16) as i16;
      }
      15 => {
        self.push_sx(value as i16);
        self.push_sy((value >> 16) as i16);
      }
      16..=19 => self.sz_fifo[destination - 16] = value as u16,
      20..=22 => {
        self.rgb_fifo[destination - 20].r = value as u8;
        self.rgb_fifo[destination - 20].g = (value >> 8) as u8;
        self.rgb_fifo[destination - 20].b = (value >> 16) as u8;
        self.rgb_fifo[destination - 20].c = (value >> 24) as u8;
      }
      23 => self.res1 = value,
      24..=27 => self.mac[destination - 24] = value as i32,
      28 => {
        self.ir[1] = ((value & 0x1f) << 7) as i16;
        self.ir[2] = (((value >> 5) & 0x1f) << 7) as i16;
        self.ir[3] = (((value >> 10) & 0x1f) << 7) as i16;
      }
      29 => (),
      30 => {
        self.lzcs = value as i32;
        self.lzcr = Self::get_num_leading_bits(self.lzcs);
      }
      31 => (),
      _ => panic!("unhandled destination received: {destination}")
    }
  }

  pub fn write_control(&mut self, destination: usize, value: u32) {

    match destination {
      0 => {
        self.rotation[0][0] = value as i16;
        self.rotation[0][1] = (value >> 16) as i16;
      }
      1 => {
        self.rotation[0][2] = value as i16;
        self.rotation[1][0] = (value >> 16) as i16;
      }
      2 => {
        self.rotation[1][1] = value as i16;
        self.rotation[1][2] = (value >> 16) as i16;
      }
      3 => {
        self.rotation[2][0] = value as i16;
        self.rotation[2][1] = (value >> 16) as i16;
      }
      4 => self.rotation[2][2] = value as i16,
      5 => self.tr.0 = value as i32,
      6 => self.tr.1 = value as i32,
      7 => self.tr.2 = value as i32,
      8 => {
        self.light[0][0] = value as i16;
        self.light[0][1] = (value >> 16) as i16;
      }
      9 => {
        self.light[0][2] = value as i16;
        self.light[1][0] = (value >> 16) as i16;
      }
      10 => {
        self.light[1][1] = value as i16;
        self.light[1][2] = (value >> 16) as i16;
      }
      11 => {
        self.light[2][0] = value as i16;
        self.light[2][1] = (value >> 16) as i16;
      }
      12 => self.light[2][2] = value as i16,
      13 => self.bk.0 = value as i32,
      14 => self.bk.1 = value as i32,
      15 => self.bk.2 = value as i32,
      16 => {
        self.color[0][0] = value as i16;
        self.color[0][1] = (value >> 16) as i16;
      }
      17 => {
        self.color[0][2] = value as i16;
        self.color[1][0] = (value >> 16) as i16;
      }
      18 => {
        self.color[1][1] = value as i16;
        self.color[1][2] = (value >> 16) as i16;
      }
      19 => {
        self.color[2][0] = value as i16;
        self.color[2][1] = (value >> 16) as i16;
      }
      20 => self.color[2][2] = value as i16,
      21 => self.fc.0 = value as i32,
      22 => self.fc.1 = value as i32,
      23 => self.fc.2 = value as i32,
      24 => self.ofx = value as i32,
      25 => self.ofy = value as i32,
      26 => self.h = value as u16,
      27 => self.dqa = value as i16,
      28 => self.dqb = value as i32,
      29 => self.zsf3 = value as i16,
      30 => self.zsf4 = value as i16,
      31 => {
        self.flags = value & 0x7fff_f000;

        if (value & 0x7f87e000) != 0 {
          self.flags |= 0x8000_0000;
        }
      }
      _ => panic!("unhandled destination received: {destination}")
    }
  }

  fn get_num_leading_bits(num: i32) -> i32 {
    let leading_bit = ((num as u32) >> 31) & 0b1;

    let mut num_bits = 1;

    for i in 1..32 {
      let bit = ((num as u32) >> (31 - i)) & 0b1;

      if bit == leading_bit {
        num_bits += 1;
      } else {
        break;
      }
    }

    num_bits
  }

  pub fn read_control(&self, destination: usize) -> u32 {
    match destination {
      0 => {
        (self.rotation[0][0] as u16 as u32) | (self.rotation[0][1] as u16 as u32) << 16
      }
      1 => {
        (self.rotation[0][2] as u16 as u32) | (self.rotation[1][0] as u16 as u32) << 16
      }
      2 => {
        (self.rotation[1][1] as u16 as u32) | (self.rotation[1][2] as u16 as u32) << 16
      }
      3 => {
        (self.rotation[2][0] as u16 as u32) | (self.rotation[2][1] as u16 as u32) << 16
      }
      4 => self.rotation[2][2] as u32,
      5 => self.tr.0 as u32,
      6 => self.tr.1 as u32,
      7 => self.tr.2 as u32,
      8 => {
        (self.light[0][0] as u16 as u32) | (self.light[0][1] as u16 as u32) << 16
      }
      9 => {
        (self.light[0][2] as u16 as u32) | (self.light[1][0] as u16 as u32) << 16
      }
      10 => {
        (self.light[1][1] as u16 as u32) | (self.light[1][2] as u16 as u32) << 16
      }
      11 => {
        (self.light[2][0] as u16 as u32) | (self.light[2][1] as u16 as u32) << 16
      }
      12 => self.light[2][2] as u32,
      13 => self.bk.0 as u32,
      14 => self.bk.1 as u32,
      15 => self.bk.2 as u32,
      16 => {
        (self.color[0][0] as u16 as u32) | (self.color[0][1] as u16 as u32) << 16
      }
      17 => {
        (self.color[0][2] as u16 as u32) | (self.color[1][0] as u16 as u32) << 16
      }
      18 => {
        (self.color[1][1] as u16 as u32) | (self.color[1][2] as u16 as u32) << 16
      }
      19 => {
        (self.color[2][0] as u16 as u32) | (self.color[2][1] as u16 as u32) << 16
      }
      20 => self.color[2][2] as u32,
      21 => self.fc.0 as u32,
      22 => self.fc.1 as u32,
      23 => self.fc.2 as u32,
      24 => self.ofx as u32,
      25 => self.ofy as u32,
      26 => self.h as u32,
      27 => self.dqa as u32,
      28 => self.dqb as u32,
      29 => self.zsf3 as u32,
      30 => self.zsf4 as u32,
      31 => self.flags,
      _ => unreachable!("can't happen")
    }
  }
}