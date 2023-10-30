use super::{CPU, instruction::Instruction};

impl CPU {

  /**
   * returns true if operation is non-branching, false otherwise
   */
  pub fn execute(&mut self, instr: Instruction) -> bool {
    let op_code = instr.op_code();

    println!("received op code {}", self.parse_op_code(op_code));

    match op_code {
      0 => {
        let op_code = instr.op_code_special();

        println!("received secondary op {}", self.parse_secondary(op_code));

        match op_code {
          0 => self.sll(instr),
          0x25 => self.or(instr),
          _ => todo!("invalid or unimplemented special op code: {:06b}", op_code)
        }

      }
      0x2 => self.j(instr),
      0x5 => self.bne(instr),
      0x8 => self.addi(instr),
      0x9 => self.addiu(instr),
      0x10 => self.execute_cop0(instr),
      0xd => self.ori(instr),
      0xf => self.lui(instr),
      0x2b => self.swi(instr),
      _ => todo!("invalid or unimplemented op code: {:06b}", op_code)
    }
  }

  fn execute_cop0(&mut self, instr: Instruction) -> bool {
    let op_code = instr.cop0_code();

    match op_code {
      0b00100 => self.mtc0(instr),
      _ => todo!("cop0 instruction not implemented yet")
    }

    true
  }

  fn bne(&mut self, instr: Instruction) -> bool {
    let offset = instr.immediate_signed();

    if self.r[instr.rs()] != self.r[instr.rt()] {
      self.branch(offset);
      false
    } else {
      true
    }
  }

  fn branch(&mut self, offset: u32) {
    self.previous_pc = self.pc.wrapping_sub(4);

    let offset = offset << 2;

    // without the wrapping sub the PC would be one instruction ahead
    self.pc = self.pc.wrapping_add(offset).wrapping_sub(4);
  }

  fn mtc0(&mut self, instr: Instruction) {

    let cop0_reg = instr.rd();


    match cop0_reg {
      12 => self.sr = self.r[instr.rt()],
      _ => todo!("cop0 register not implemented in mtc0: {}", cop0_reg)
    }

  }

  fn j(&mut self, instr: Instruction) -> bool {
    self.previous_pc = self.pc.wrapping_sub(4);
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

  fn or(&mut self, instr: Instruction) -> bool {
    let result = self.r[instr.rs()] | self.r[instr.rt()];

    println!("rs = {} rt = {} rd = {}", instr.rs(), instr.rt(), instr.rd());

    self.set_reg(instr.rd(), result);

    true
  }

  fn addi(&mut self, instr: Instruction) -> bool {
    if let Some(result) = (self.r[instr.rs()] as i32).checked_add(instr.immediate_signed() as i32) {
      self.set_reg(instr.rt(), result as u32);
    } else {
      // handle exceptions here later
      todo!("unhandled overflow occurred for instruction ADDI");
    }

    true
  }

  fn addiu(&mut self, instr: Instruction) -> bool {
    let result = self.r[instr.rs()].wrapping_add(instr.immediate_signed());
    self.set_reg(instr.rt(), result);

    true
  }

  fn swi(&mut self, instr: Instruction) -> bool {
    if self.sr & 0x10000 == 0 {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      let value = self.r[instr.rt()];

      self.bus.mem_write_32(address, value);
    } else {
      println!("ignoring writes to cache");
    }

    true
  }

  fn parse_secondary(&self, op_code: u32) -> &'static str {
    match op_code {
      0 => "SLL",
      0x25 => "OR",
      _ => todo!("Invalid or unimplemented special op: {:06b}", op_code)
    }
  }

  fn parse_op_code(&self, op_code: u32) -> &'static str {
    match op_code {
      0 => "Special",
      0x2 => "J",
      0x5 => "BNE",
      0x8 => "ADDI",
      0x9 => "ADDIU",
      0x10 => "COP0",
      0xd => "ORI",
      0xf => "LUI",
      0x2b => "SWI",
      _ => todo!("invalid or unimplemented op code: {:06b}", op_code)
    }
  }
}