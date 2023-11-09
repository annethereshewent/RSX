#[derive(Clone, Copy)]
pub struct Timer {
  pub value: u16,
  pub target_value: u16,
  pub mode: u16
}

impl Timer {
  pub fn new() -> Self {
    Self {
      value: 0,
      target_value: 0,
      mode: 0
    }
  }
}