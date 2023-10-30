use super::{CPU, instruction::Instruction};

impl CPU {

  /**
   * returns true on branches and false otherwise
   */
  pub fn execute(&mut self, instr: Instruction) -> bool {
    let op_code = instr.op_code();

    println!("received op code {}", self.parse_op_code(op_code));

    match op_code {
      0 => {
        let op_code = instr.op_code_special();

        println!("received special op {}", self.parse_special(op_code));

        match op_code {
          0 => self.sll(instr),
          _ => todo!("invalid or unimplemented special op code: {:06b}", op_code)
        }

      }
      0x2 => self.j(instr),
      0x9 => self.addiu(instr),
      0xd => self.ori(instr),
      0xf => self.lui(instr),
      0x2b => self.swi(instr),
      _ => todo!("invalid or unimplemented op code: {:06b}", op_code)
    }
  }

  fn j(&mut self, instr: Instruction) -> bool {
    println!("jump address = {:x}, upper pc = {:x}", instr.j_imm() << 2, self.pc & 0xf0000000);
    self.previous_pc = self.pc - 4;
    self.pc = (self.pc & 0xf000_0000) | (instr.j_imm() << 2);

    false
  }

  fn sll(&mut self, instr: Instruction) -> bool {
    let result = self.r[instr.rt()] << instr.imm5();

    self.set_reg(instr.rd(), result);

    true
  }

  fn lui(&mut self, instr: Instruction) -> bool {
    self.set_reg(instr.rt(), instr.immediate() << 16);

    true
  }

  fn ori(&mut self, instr: Instruction) -> bool {
    let result = self.r[instr.rs()] | instr.immediate();

    self.set_reg(instr.rt(), result);

    true
  }

  fn addiu(&mut self, instr: Instruction) -> bool {
    let result = self.r[instr.rs()].wrapping_add(instr.immediate_signed());
    self.set_reg(instr.rt(), result);

    true
  }

  fn swi(&mut self, instr: Instruction) -> bool {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let value = self.r[instr.rt()];

    self.bus.mem_write_32(address, value);

    true
  }

  fn parse_special(&self, op_code: u32) -> &'static str {
    match op_code {
      0 => "SLL",
      _ => todo!("Invalid or unimplemented special op: {:06b}", op_code)
    }
  }

  fn parse_op_code(&self, op_code: u32) -> &'static str {
    match op_code {
      0 => "Special",
      0x2 => "J",
      0x9 => "ADDIU",
      0xd => "ORI",
      0xf => "LUI",
      0x2b => "SWI",
      _ => todo!("invalid or unimplemented op code: {:06b}", op_code)
    }
  }
}