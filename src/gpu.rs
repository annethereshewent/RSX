use self::gpu_stat_register::GpuStatRegister;

pub mod gpu_stat_register;

pub struct GPU {
  pub stat: GpuStatRegister,
  texture_rectangle_x_flip: bool,
  texture_rectangle_y_flip: bool
}

impl GPU {
  pub fn new() -> Self {
    Self {
      stat: GpuStatRegister::new(),
      texture_rectangle_x_flip: false,
      texture_rectangle_y_flip: false
    }
  }

  pub fn gp0(&mut self, val: u32) {
    let op_code = (val >> 24) & 0xff;

    match op_code {
      0x00 => (), // NOP
      0xe1 => self.gp0_draw_mode(val),
      _ => todo!("invalid or unsupported GP0 command: {:02x}", op_code)
    }
  }

  pub fn gp1(&mut self, val: u32) {

  }

  fn gp0_draw_mode(&mut self, val: u32) {
    self.stat.update_draw_mode(val);

    self.texture_rectangle_x_flip = ((val >> 12) & 0b1) == 1;
    self.texture_rectangle_y_flip = ((val >> 13) & 0b1) == 1;
  }
}