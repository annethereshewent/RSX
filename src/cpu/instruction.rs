pub struct Instruction(u32);

impl Instruction {
  pub fn new(instr: u32) -> Self {
    Self(instr)
  }
  pub fn immediate(&self) -> u32 {
    self.0 & 0xffff
  }

  pub fn rs(&self) -> usize {
    ((self.0 >> 21) & 0b11111) as usize
  }

  pub fn cop0_code(&self) -> u32 {
    (self.0 >> 21) & 0b11111
  }

  pub fn op_code_secondary(&self) -> u32 {
    self.0 & 0b111111
  }

  pub fn imm5(&self) -> u32 {
    ((self.0 >> 6)) & 0b11111
  }

  pub fn j_imm(&self) -> u32 {
    self.0 & 0x3ff_ffff
  }

  pub fn immediate_signed(&self) -> u32 {
    (self.0 & 0xffff) as i16 as u32
  }

  pub fn rt(&self) -> usize {
    ((self.0 >> 16) & 0b11111) as usize
  }

  pub fn rd(&self) -> usize {
    ((self.0 >> 11) & 0b11111) as usize
  }

  pub fn op_code(&self) -> u32 {
    self.0 >> 26
  }
}