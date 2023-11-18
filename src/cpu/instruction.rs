pub struct Instruction(u32);

impl Instruction {

  pub fn to_u32(&self) -> u32 {
    self.0
  }
  pub fn new(instr: u32) -> Self {
    Self(instr)
  }
  pub fn immediate(&self) -> u32 {
    self.0 & 0xffff
  }

  pub fn rs(&self) -> usize {
    ((self.0 >> 21) & 0b11111) as usize
  }

  pub fn cop_code(&self) -> u32 {
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

  pub fn cop2_command(&self) -> u32 {
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

  pub fn bcond(&self) -> u32 {
    (self.0 >> 16) & 0b1
  }

  pub fn should_link(&self) -> bool {
    (self.0 >> 20) & 0b1 == 1
  }

  pub fn cop0_lower_bits(&self) -> u32 {
    self.0 & 0x3f
  }
}