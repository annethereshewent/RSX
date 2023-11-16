use super::instruction::Instruction;

struct GteRgb {
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
  rgbc: GteRgb,
  otz: u16,
  ir: [i16; 4],
  flags: u32,
  sf: usize,
  mx: usize,
  sv: usize,
  cv: usize,
  lm: bool
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
      rgbc: GteRgb {
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
      lm: false
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
      0x30 => self.triple_perspective_transform(),
      _ => panic!("unimplemented op code for gte: {:x}", op_code)
    }
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

    panic!("todo!");
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
      8 => self.ir[0] = value as i16,
      9 => self.ir[1] = value as i16,
      10 => self.ir[2] = value as i16,
      11 => self.ir[3] = value as i16,

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