#[derive(Clone, Copy)]
pub struct DmaBlockControlRegister {
  pub val: u32
}

impl DmaBlockControlRegister {
  pub fn new() -> Self {
    Self {
      val: 0
    }
  }

  pub fn block_size(&self) -> u32 {
    self.val & 0xffff
  }

  pub fn block_count(&self) -> u32 {
    self.val >> 16
  }
}