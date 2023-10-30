use super::{CPU, instruction::Instruction};

impl CPU {
  pub fn execute(&mut self, instr: Instruction) {
    let op_code = instr.op_code();

    println!("received op code {}", self.parse_op_code(op_code));

    match op_code {
      0b001111 => self.lui(instr),
      0b001101 => self.ori(instr),
      _ => todo!("invalid or unimplemented op code")
    }
  }

  fn lui(&mut self, instr: Instruction) {
    self.set_reg(instr.rt(), instr.immediate() << 16);
  }

  fn ori(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] | instr.immediate();

    self.set_reg(instr.rt(), result);
  }

  fn parse_op_code(&self, op_code: u32) -> &'static str {
    match op_code {
      0b001111 => "LUI",
      0b001101 => "ORI",
      _ => todo!("invalid or unimplemented op code")
    }
  }
}