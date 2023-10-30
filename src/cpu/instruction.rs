pub struct Instruction(u32);

impl Instruction {
  pub fn new(instr: u32) -> Self {
    Self(instr)
  }
  pub fn immediate(&self) -> u32 {
    let Instruction(instr) = self;

    instr & 0xffff
  }

  pub fn rs(&self) -> usize {
    let Instruction(instr) = self;
    ((instr >> 21) & 0b11111) as usize
  }

  pub fn imm5(&self) -> u32 {
    let Instruction(instr) = self;
    ((instr >> 6)) & 0b11111
  }

  pub fn rt(&self) -> usize {
    let Instruction(instr) = self;
    ((instr >> 16) & 0b11111) as usize
  }

  pub fn rd(&self) -> usize {
    let Instruction(instr) = self;
    ((instr >> 11) & 0b11111) as usize
  }

  pub fn op_code(&self) -> u32 {
    let Instruction(instr) = self;
    instr >> 26
  }
}