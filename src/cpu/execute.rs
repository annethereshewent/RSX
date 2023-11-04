use super::{CPU, instruction::Instruction, Cause};


const RA_REGISTER: usize = 31;

impl CPU {
  pub fn execute(&mut self, instr: Instruction) {
    let op_code = instr.op_code();

    // println!("received op code {}", self.parse_op_code(op_code));

    match op_code {
      0 => {
        let op_code = instr.op_code_secondary();

       // println!("received secondary op {}", self.parse_secondary(op_code));

        match op_code {
          0 => self.sll(instr),
          0x2 => self.srl(instr),
          0x3 => self.sra(instr),
          0x4 => self.sllv(instr),
          0x6 => self.srlv(instr),
          0x7 => self.srav(instr),
          0x8 => self.jr(instr),
          0x9 => self.jalr(instr),
          0xc => self.syscall(instr),
          0xd => self.op_break(instr),
          0x10 => self.mfhi(instr),
          0x11 => self.mthi(instr),
          0x12 => self.mflo(instr),
          0x13 => self.mtlo(instr),
          0x18 => self.mult(instr),
          0x19 => self.multu(instr),
          0x1a => self.div(instr),
          0x1b => self.divu(instr),
          0x20 => self.add(instr),
          0x21 => self.addu(instr),
          0x22 => self.sub(instr),
          0x23 => self.subu(instr),
          0x24 => self.and(instr),
          0x26 => self.xor(instr),
          0x25 => self.or(instr),
          0x27 => self.nor(instr),
          0x2a => self.slt(instr),
          0x2b => self.sltu(instr),
          _ => panic!("illegal secondary op code: {:02x}", op_code)
        }

      }
      0x1 => {
        match instr.bcond() {
          0 => self.bltz(instr),
          1 => self.bgez(instr),
          _ => unreachable!("can't happen")
        }
      }
      0x2 => self.j(instr),
      0x3 => self.jal(instr),
      0x4 => self.beq(instr),
      0x5 => self.bne(instr),
      0x6 => self.blez(instr),
      0x7 => self.bgtz(instr),
      0x8 => self.addi(instr),
      0x9 => self.addiu(instr),
      0xa => self.slti(instr),
      0xb => self.sltiu(instr),
      0xc => self.andi(instr),
      0xd => self.ori(instr),
      0xe => self.xori(instr),
      0xf => self.lui(instr),
      0x10 => self.execute_cop0(instr),
      0x11 => self.exception(Cause::CoprocessorError),
      0x12 => self.execute_cop2(instr),
      0x13 => self.exception(Cause::CoprocessorError),
      0x20 => self.lb(instr),
      0x21 => self.lh(instr),
      0x22 => self.lwl(instr),
      0x23 => self.lw(instr),
      0x24 => self.lbu(instr),
      0x25 => self.lhu(instr),
      0x26 => self.lwr(instr),
      0x28 => self.sb(instr),
      0x29 => self.sh(instr),
      0x2a => self.swl(instr),
      0x2b => self.swi(instr),
      0x2e => self.swr(instr),
      0x30..=0x31 | 0x33 => self.exception(Cause::CoprocessorError),
      0x32 => self.lwc2(instr),
      0x38..=0x39 | 0x3b => self.exception(Cause::CoprocessorError),
      0x3a => self.swc2(instr),
      _ => panic!("illegal op code: {:02x}", op_code)
    }
  }

  fn sh(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      let value = self.r[instr.rt()];

      if address & 0b1 == 0 {
        self.bus.mem_write_16(address, value as u16);
      } else {
        self.exception(Cause::StoreAddressError);
      }
    } else {
      // println!("ignoring writes to cache");
    }
  }

  fn sb(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      let value = self.r[instr.rt()];

      self.bus.mem_write_8(address, value as u8);
    } else {
      // println!("ignoring writes to cache");
    }
  }

  fn lb(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      let value = self.bus.mem_read_8(address);

      self.delayed_load = Some(value as i8 as i32 as u32);
      self.delayed_register = instr.rt();
    } else {
      // println!("cache not implemented yet for loads");
    }
  }

  fn lbu(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      let value = self.bus.mem_read_8(address);

      self.delayed_load = Some(value as u32);
      self.delayed_register = instr.rt();
    } else {
      // println!("cache not implemented yet for loads");
    }
  }

  fn lhu(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      if address & 0b1 == 0 {
        let value = self.bus.mem_read_16(address);

        self.delayed_load = Some(value as u32);
        self.delayed_register = instr.rt();
      } else {
        self.exception(Cause::LoadAddressError);
      }
    } else {
      // println!("cache not implemented yet for loads");
    }
  }

  fn lh(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      if address & 0b1 == 0 {
        let value = self.bus.mem_read_16(address) as i16;

        self.delayed_load = Some(value as u32);
        self.delayed_register = instr.rt();
      } else {
        self.exception(Cause::LoadAddressError);
      }
    } else {
      // println!("cache not implemented yet for loads");
    }
  }

  fn execute_cop0(&mut self, instr: Instruction) {
    let op_code = instr.cop0_code();

    match op_code {
      0b00000 => self.mfc0(instr),
      0b00100 => self.mtc0(instr),
      0b10000 => self.rfe(instr),
      _ => todo!("cop0 instruction not implemented yet")
    }
  }

  fn execute_cop2(&mut self, _instr: Instruction) {
    todo!("unhandled instruction to cop2 processor");
  }

  fn lwc2(&mut self, _instr: Instruction) {
    todo!("load word to coprocessor 2 not implemented");
  }

  fn swc2(&mut self, _instr: Instruction) {
    todo!("store word to coprocessor 2 not implemented");
  }

  fn bne(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, self.r[instr.rs()] != self.r[instr.rt()]);
  }

  fn bltz(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();
    if instr.should_link() {
      self.set_reg(RA_REGISTER, self.next_pc);
    }

    self.branch_if(offset, (self.r[instr.rs()] as i32) < 0);
  }

  fn bgez(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();
    if instr.should_link() {
      self.set_reg(RA_REGISTER, self.next_pc);
    }

    self.branch_if(offset, (self.r[instr.rs()] as i32) >= 0);
  }

  fn bgtz(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, (self.r[instr.rs()] as i32) > 0);
  }

  fn blez(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, (self.r[instr.rs()] as i32) <= 0);
  }

  fn beq(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, self.r[instr.rs()] == self.r[instr.rt()]);
  }

  fn branch_if(&mut self, offset: u32, condition: bool) {
    if condition {
      self.branch(offset);
    }
  }

  fn branch(&mut self, offset: u32) {
    let offset = offset << 2;

    self.next_pc = self.pc.wrapping_add(offset);

    self.branch = true;
  }

  fn sltu(&mut self, instr: Instruction) {
    let result: u32 = if self.r[instr.rs()] < self.r[instr.rt()] {
      1
    } else {
      0
    };

    self.set_reg(instr.rd(), result);
  }

  fn slt(&mut self, instr: Instruction) {
    let result: u32 = if (self.r[instr.rs()] as i32) < (self.r[instr.rt()] as i32) {
      1
    } else {
      0
    };

    self.set_reg(instr.rd(), result);
  }

  fn sltiu(&mut self, instr: Instruction) {
    let result: u32 = if self.r[instr.rs()] < instr.immediate_signed() {
      1
    } else {
      0
    };

    self.set_reg(instr.rt(), result);
  }

  fn slti(&mut self, instr: Instruction) {
    let result: u32 = if (self.r[instr.rs()] as i32) < (instr.immediate_signed() as i32) {
      1
    } else {
      0
    };

    self.set_reg(instr.rt(), result);
  }

  fn mfc0(&mut self, instr: Instruction) {
    self.delayed_register = instr.rt();
    self.delayed_load = match instr.rd() {
      12 => Some(self.cop0.sr),
      13 => Some(self.cop0.cause),
      14 => Some(self.cop0.epc),
      _ => panic!("unhandled read from cop0 register: {}", instr.rd())
    }
  }

  fn mtc0(&mut self, instr: Instruction) {

    let cop0_reg = instr.rd();
    let value = self.r[instr.rt()];

    match cop0_reg {
      3 | 5 | 6 | 7 | 9 | 11 => {
        if value != 0 {
          panic!("unhandled write to debug registers");
        }
      }
      12 => self.cop0.sr = value,
      13 => self.cop0.cause = value,
      _ => todo!("cop0 register not implemented in mtc0: {}", cop0_reg)
    }

  }

  fn lw(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      if address & 0b11 == 0 {
        self.delayed_load = Some(self.bus.mem_read_32(address));
        self.delayed_register = instr.rt();
      } else {
        self.exception(Cause::LoadAddressError);
      }
    } else {
      // println!("cache not implemented yet for loads");
    }
  }

  fn lwl(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let mut result = if let Some(val) = self.get_out_register_val(instr.rt()) {
      val
    } else {
      self.r[instr.rt()]
    };

    let aligned_address = address & !0x3;
    let aligned_word = self.bus.mem_read_32(aligned_address);

    result = match address & 0x3 {
      0 => (result & 0xffffff) | (aligned_word << 24),
      1 => (result & 0xffff) | (aligned_word << 16),
      2 => (result & 0xff) | (aligned_word << 8),
      3 => aligned_word,
      _ => unreachable!("can't happen")
    };

    self.delayed_load = Some(result);
    self.delayed_register = instr.rt();
  }

  fn lwr(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let mut result = if let Some(val) = self.get_out_register_val(instr.rt()) {
      val
    } else {
      self.r[instr.rt()]
    };

    let aligned_address = address & !0x3;
    let aligned_word = self.bus.mem_read_32(aligned_address);

    result = match address & 0x3 {
      0 => aligned_word,
      1 => (result & 0xff000000) | (aligned_word >> 8),
      2 => (result & 0xffff0000) | (aligned_word >> 16),
      3 => (result & 0xffffff00) | (aligned_word >> 24),
      _ => unreachable!("can't happen")
    };

    self.delayed_load = Some(result);
    self.delayed_register = instr.rt();
  }

  fn j(&mut self, instr: Instruction) {
    self.next_pc = (self.pc & 0xf000_0000) | (instr.j_imm() << 2);

    self.branch = true;
  }

  fn jal(&mut self, instr: Instruction) {
    self.set_reg(RA_REGISTER, self.next_pc);

    self.j(instr);
  }

  fn jalr(&mut self, instr: Instruction) {
    self.set_reg(instr.rd(), self.next_pc);

    self.next_pc = self.r[instr.rs()];
    self.branch = true;
  }

  fn jr(&mut self, instr: Instruction) {
    self.next_pc = self.r[instr.rs()];
    self.branch = true;
  }

  fn sll(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] << instr.imm5();

    self.set_reg(instr.rd(), result);
  }

  fn sllv(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] << (self.r[instr.rs()] & 0x1f);

    self.set_reg(instr.rd(), result);
  }

  fn srlv(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] >> (self.r[instr.rs()] & 0x1f);

    self.set_reg(instr.rd(), result);
  }

  fn sra(&mut self, instr: Instruction) {
    let result = (self.r[instr.rt()] as i32) >> instr.imm5();

    self.set_reg(instr.rd(), result as u32);
  }

  fn srav(&mut self, instr: Instruction) {
    let result = (self.r[instr.rt()] as i32) >> (self.r[instr.rs()] & 0x1f);

    self.set_reg(instr.rd(), result as u32);
  }

  fn srl(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] >> instr.imm5();

    self.set_reg(instr.rd(), result);
  }

  fn lui(&mut self, instr: Instruction) {
    self.set_reg(instr.rt(), instr.immediate() << 16);
  }

  fn mflo(&mut self, instr: Instruction) {
    self.set_reg(instr.rd(), self.low);
  }

  fn mtlo(&mut self, instr: Instruction) {
    self.low = self.r[instr.rs()];
  }

  fn mfhi(&mut self, instr: Instruction) {
    self.set_reg(instr.rd(), self.hi);
  }

  fn mthi(&mut self, instr: Instruction) {
    self.hi = self.r[instr.rs()];
  }

  fn ori(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] | instr.immediate();

    self.set_reg(instr.rt(), result);
  }

  fn or(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] | self.r[instr.rt()];

    self.set_reg(instr.rd(), result);
  }

  fn xori(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] ^ instr.immediate();

    self.set_reg(instr.rt(), result);
  }

  fn xor(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] ^ self.r[instr.rt()];

    self.set_reg(instr.rd(), result);
  }

  fn nor(&mut self, instr: Instruction) {
    let result = !(self.r[instr.rs()] | self.r[instr.rt()]);

    self.set_reg(instr.rd(), result);
  }

  fn andi(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] & instr.immediate();

    self.set_reg(instr.rt(), result);
  }

  fn and(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] & self.r[instr.rt()];

    self.set_reg(instr.rd(), result);
  }

  fn addi(&mut self, instr: Instruction) {
    if let Some(result) = (self.r[instr.rs()] as i32).checked_add(instr.immediate_signed() as i32) {
      self.set_reg(instr.rt(), result as u32);
    } else {
      self.exception(Cause::Overflow);
    }
  }

  fn add(&mut self, instr: Instruction) {
    if let Some(result) = (self.r[instr.rs()] as i32).checked_add(self.r[instr.rt()] as i32) {
      self.set_reg(instr.rd(), result as u32);
    } else {
      self.exception(Cause::Overflow);
    }
  }

  fn addiu(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()].wrapping_add(instr.immediate_signed());
    self.set_reg(instr.rt(), result);
  }

  fn addu(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()].wrapping_add(self.r[instr.rt()]);

    self.set_reg(instr.rd(), result);
  }

  fn subu(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()].wrapping_sub(self.r[instr.rt()]);

    self.set_reg(instr.rd(), result);
  }

  fn sub(&mut self, instr: Instruction) {
    if let Some(result) = (self.r[instr.rs()] as i32).checked_sub(self.r[instr.rt()] as i32) {
      self.set_reg(instr.rd(), result as u32);
    } else {
      self.exception(Cause::Overflow);
    }
  }

  fn multu(&mut self, instr: Instruction) {
    let a = self.r[instr.rs()] as u64;
    let b = self.r[instr.rt()] as u64;

    let result = a * b;

    self.low = result as u32;
    self.hi = (result >> 32) as u32;
  }

  fn mult(&mut self, instr: Instruction) {
    let a = self.r[instr.rs()] as i32 as i64;
    let b = self.r[instr.rt()] as i32 as i64;

    let result = a * b;

    self.low = result as u32;
    self.hi = (result >> 32) as u32;
  }

  fn div(&mut self, instr: Instruction) {
    let numerator = self.r[instr.rs()] as i32;
    let denominator = self.r[instr.rt()] as i32;

    if denominator == 0 {
      self.low = if numerator >= 0 { 0xffffffff } else { 1 };
      self.hi = numerator as u32;
    } else if (numerator as u32)  == 0x80000000 && denominator == -1 {
      self.low = numerator as u32;
      self.hi = 0;
    } else {
      self.low = (numerator / denominator) as u32;
      self.hi = (numerator % denominator) as u32;
    }
  }

  fn divu(&mut self, instr: Instruction) {
    let numerator = self.r[instr.rs()];
    let denominator = self.r[instr.rt()];

    if denominator == 0 {
      self.low = 0xffffffff;
      self.hi = numerator;
    }  else {
      self.low = numerator / denominator;
      self.hi = numerator % denominator;
    }
  }

  fn swl(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let aligned_address = address & !0x3;

    let val = self.r[instr.rt()];
    let mem_value = self.bus.mem_read_32(aligned_address);

    let result = match address & 0x3 {
      0 => (mem_value & 0xffffff00) | (val >> 24),
      1 => (mem_value & 0xffff0000) | (val >> 16),
      2 => (mem_value & 0xff000000) | (val >> 8),
      3 => val,
      _ => unreachable!("can't happen")
    };

    self.bus.mem_write_32(address, result);
  }

  fn swr(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let aligned_address = address & !0x3;

    let val = self.r[instr.rt()];
    let mem_value = self.bus.mem_read_32(aligned_address);

    let result = match address & 0x3 {
      0 => val,
      1 => (mem_value & 0xff) | (val << 8),
      2 => (mem_value & 0xffff) | (val << 16),
      3 => (mem_value & 0xffffff) | (val << 24),
      _ => unreachable!("can't happen")
    };

    self.bus.mem_write_32(address, result);
  }

  fn swi(&mut self, instr: Instruction) {
    if self.cop0.is_cache_disabled() {
      let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

      let value = self.r[instr.rt()];

      if address & 0b11 == 0 {
        self.bus.mem_write_32(address, value);
      } else {
        self.exception(Cause::StoreAddressError);
      }
    } else {
      // println!("ignoring writes to cache");
    }
  }

  fn syscall(&mut self, _instr: Instruction) {
    self.exception(Cause::SysCall);
  }

  fn op_break(&mut self, _instr: Instruction) {
    self.exception(Cause::Break);
  }

  fn rfe(&mut self, instr: Instruction) {
    if instr.cop0_lower_bits() != 0b010000 {
      panic!("illegal cop0 instruction received")
    }
    self.cop0.return_from_exception();
  }

  fn parse_secondary(&self, op_code: u32) -> &'static str {
    match op_code {
      0x0 => "SLL",
      0x2 => "SRL",
      0x3 => "SRA",
      0x4 => "SLLV",
      0x6 => "SRLV",
      0x7 => "SRAV",
      0x8 => "JR",
      0x9 => "JALR",
      0xc => "SYSCALL",
      0xd => "BREAK",
      0x10 => "MFHI",
      0x11 => "MTHI",
      0x12 => "MFLO",
      0x13 => "MTLO",
      0x18 => "MULT",
      0x19 => "MULTU",
      0x1a => "DIV",
      0x1b => "DIVU",
      0x20 => "ADD",
      0x21 => "ADDU",
      0x22 => "SUB",
      0x23 => "SUBU",
      0x24 => "AND",
      0x25 => "OR",
      0x26 => "XOR",
      0x27 => "NOR",
      0x2a => "SLT",
      0x2b => "SLTU",
      _ => todo!("Invalid or unimplemented secondary op: {:03x}", op_code)
    }
  }

  fn parse_op_code(&self, op_code: u32) -> &'static str {
    match op_code {
      0x0 => "Secondary",
      0x1 => "BcondZ",
      0x2 => "J",
      0x3 => "JAL",
      0x4 => "BEQ",
      0x5 => "BNE",
      0x6 => "BLEZ",
      0x7 => "BGTZ",
      0x8 => "ADDI",
      0x9 => "ADDIU",
      0xa => "SLTI",
      0xb => "SLTIU",
      0xc => "ANDI",
      0xd => "ORI",
      0xe => "XORI",
      0xf => "LUI",
      0x10 => "COP0",
      0x11 => "COP1",
      0x12 => "COP2",
      0x13 => "COP3",
      0x20 => "LB",
      0x21 => "LH",
      0x22 => "LWL",
      0x23 => "LW",
      0x24 => "LBU",
      0x25 => "LHU",
      0x26 => "LWR",
      0x28 => "SB",
      0x29 => "SH",
      0x2a => "SWL",
      0x2b => "SWI",
      0x2e => "SWR",
      0x30 => "LWC0",
      0x31 => "LWC1",
      0x32 => "LWC2",
      0x33 => "LWC3",
      0x38 => "SWC0",
      0x39 => "SWC1",
      0x3a => "SWC2",
      0x3b => "SWC3",
      _ => todo!("invalid or unimplemented op code: {:03x}", op_code)
    }
  }
}