#[derive(Clone, Copy)]
pub enum HighInput {
  ButtonL2 = 0,
  ButtonR2 = 1,
  ButtonL1 = 2,
  ButtonR1 = 3,
  ButtonTriangle = 4,
  ButtonCircle = 5,
  ButtonCross = 6,
  ButtonSquare = 7
}

#[derive(Clone, Copy)]
pub enum LowInput {
  ButtonSelect = 0,
  ButtonL3 = 1,
  ButtonR3 = 2,
  ButtonStart = 3,
  ButtonUp = 4,
  ButtonRight = 5,
  ButtonDown = 6,
  ButtonLeft = 7
}

pub struct Joypad {
  pub state: usize,
  pub digital_mode: bool,
  pub rx_axis: u8,
  pub lx_axis: u8,
  pub ry_axis: u8,
  pub ly_axis: u8,
  pub low_input: u8,
  pub high_input: u8
}

impl Joypad {
  pub fn new() -> Self {
    Self {
      state: 0,
      digital_mode: true,
      rx_axis: 128,
      ry_axis: 128,
      lx_axis: 128,
      ly_axis: 128,
      low_input: 0xff,
      high_input: 0xff
    }
  }

  pub fn ack(&self) -> bool {
    self.state != 0
  }

  pub fn set_low_input(&mut self, input: u8, val: bool) {
    if val {
      self.low_input &= !(1 << input);
    } else {
      self.low_input |= 1 << input;
    }
  }

  pub fn set_high_input(&mut self, input: u8, val: bool) {
    if val {
      self.high_input &= !(1 << input);
    } else {
      self.high_input |= 1 << input;
    }
  }

  pub fn set_rightx(&mut self, value: u8) {
    self.rx_axis = value;
  }

  pub fn set_righty(&mut self, value: u8) {
    self.ry_axis = value;
  }

  pub fn set_leftx(&mut self, value: u8) {
    self.lx_axis = value;
  }

  pub fn set_lefty(&mut self, value: u8) {
    self.ly_axis = value;
  }

  pub fn reply(&mut self, command: u8) -> u8 {
    let mut reset_state = false;

    let reply = match self.state {
      0 => 0xff,
      1 => {
        if command == 0x42 {
          if self.digital_mode { 0x41 } else { 0x73 }
        } else {
          reset_state = true;
          0xff
        }
      }
      2 => 0x5a,
      3 => self.low_input,
      4 => {
        if self.digital_mode {
          reset_state = true;
        }
        self.high_input
      }
      5 => self.rx_axis,
      6 => self.ry_axis,
      7 => self.lx_axis,
      8 => {
        reset_state = true;
        self.ly_axis
      },
      _ => panic!("invalid state for controller given: {}", self.state)
    };

    self.state = if reset_state { 0 } else { self.state + 1 };

    reply
  }
}