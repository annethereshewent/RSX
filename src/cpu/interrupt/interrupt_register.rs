pub enum Interrupt {
  Vblank = 0,
  Gpu = 1,
  Cdrom = 2,
  Dma = 3,
  Timer0 = 4,
  Timer1 = 5,
  Timer2 = 6,
  Controller = 7,
  Sio = 8,
  Spu = 9,
  Lightpen = 10
}

#[derive(Copy, Clone)]
pub struct InterruptRegister {
  val: u32
}

impl InterruptRegister {
  pub fn new() -> Self {
    Self {
      val: 0
    }
  }

  pub fn set_interrupt(&mut self, interrupt: Interrupt) {
    self.val |= 1 << (interrupt as u32);
  }

  pub fn clear_interrupt(&mut self, interrupt: Interrupt) {
    self.val &= !(1 << (interrupt as u32));
  }

  pub fn write(&mut self, val: u32) {
    self.val = val;
  }

  pub fn read(&self) -> u32 {
    self.val
  }

  pub fn vblank(&self) -> bool {
    self.val & 0b1 == 1
  }

  pub fn gpu(&self) -> bool {
    (self.val >> 1) & 0b1 == 1
  }

  pub fn cdrom(&self) -> bool {
    (self.val >> 2) & 0b1 == 1
  }

  pub fn dma(&self) -> bool {
    (self.val >> 3) & 0b1 == 1
  }

  pub fn timer0(&self) -> bool {
    (self.val >> 4) & 0b1 == 1
  }

  pub fn timer1(&self) -> bool {
    (self.val >> 5) & 0b1 == 1
  }

  pub fn timer2(&self) -> bool {
    (self.val >> 6) & 0b1 == 1
  }

  pub fn controller(&self) -> bool {
    (self.val >> 7) & 0b1 == 1
  }

  pub fn sio(&self) -> bool {
    (self.val >> 8) & 0b1 == 1
  }

  pub fn spu(&self) -> bool {
    (self.val >> 9) & 0b1 == 1
  }

  pub fn lightpen(&self) -> bool {
    (self.val >> 10) & 0b1 == 1
  }
}