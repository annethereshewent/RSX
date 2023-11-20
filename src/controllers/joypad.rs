pub struct Joypad {
  state: usize,
  pub digital_mode: bool,
  pub rx_axis: u8,
  pub lx_axis: u8,
  pub ry_axis: u8,
  pub ly_axis: u8,
  pub button_square: bool,
  pub button_cross: bool,
  pub button_circle: bool,
  pub button_triangle: bool,
  pub button_up: bool,
  pub button_down: bool,
  pub button_left: bool,
  pub button_right: bool,
  pub button_select: bool,
  pub button_start: bool,
  pub button_l1: bool,
  pub button_r1: bool,
  pub button_l2: bool,
  pub button_r2: bool,
  pub button_l3: bool,
  pub button_r3: bool
}

impl Joypad {
  pub fn new() -> Self {
    Self {
      state: 0,
      digital_mode: false,
      rx_axis: 128,
      ry_axis: 128,
      lx_axis: 128,
      ly_axis: 128,
      button_square: false,
      button_cross: false,
      button_circle: false,
      button_triangle: false,
      button_up: false,
      button_down: false,
      button_left: false,
      button_right: false,
      button_select: false,
      button_start: false,
      button_l1: false,
      button_r1: false,
      button_l2: false,
      button_r2: false,
      button_l3: false,
      button_r3: false
    }
  }

  pub fn ack(&self) -> bool {
    self.state != 0
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
      3 => self.get_low_input(),
      4 => self.get_high_input(),
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

  pub fn get_low_input(&self) -> u8 {
    let mut value = self.button_l2 as u8;

    value |= (self.button_r2 as u8) << 1;
    value |= (self.button_l1 as u8) << 2;
    value |= (self.button_r1 as u8) << 3;
    value |= (self.button_triangle as u8) << 4;
    value |= (self.button_circle as u8) << 5;
    value |= (self.button_cross as u8) << 6;
    value |= (self.button_square as u8) << 7;

    value
  }

  pub fn get_high_input(&self) -> u8 {
    let mut value = self.button_select as u8;

    value |= (self.button_l3 as u8) << 1;
    value |= (self.button_r3 as u8) << 2;
    value |= (self.button_start as u8) << 3;
    value |= (self.button_up as u8) << 4;
    value |= (self.button_right as u8) << 5;
    value |= (self.button_down as u8) << 6;
    value |= (self.button_left as u8) << 7;

    value
  }
}