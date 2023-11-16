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
  rotation: [[i16; 3]; 3]
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
    }
  }

  pub fn write_data(&mut self, destination: usize, value: u32) {

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

      _ => panic!("unhandled destination received: {destination}")
    }
  }
}