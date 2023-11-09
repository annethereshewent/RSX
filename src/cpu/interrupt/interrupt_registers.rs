use super::interrupt_register::InterruptRegister;

#[derive(Copy, Clone)]
pub struct InterruptRegisters {
  pub status: InterruptRegister,
  pub mask: InterruptRegister
}

impl InterruptRegisters {
  pub fn new() -> Self {
    Self {
      status: InterruptRegister::new(),
      mask: InterruptRegister::new()
    }
  }

  pub fn acknowledge_irq(&mut self, value: u32) {
    self.status.write(self.status.read() & value);
  }

  pub fn pending(&self) -> bool {
    self.status.read() & self.mask.read() != 0
  }
}