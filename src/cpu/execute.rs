use super::{CPU, instruction::Instruction, Cause};


const RA_REGISTER: usize = 31;

const PRIMARY_OPS: [&str; 64] = [
  "",     "BcondZ", "J",    "JAL",  "BEQ",
  "BNE",  "BLEZ",   "BGTZ", "ADDI", "ADDIU",
  "SLTI", "SLTIU",  "ANDI", "ORI",  "XORI",
  "LUI",  "COP0",   "COP1", "COP2", "COP3",
  "",     "",       "",     "",     "",
  "",     "",       "",     "",     "",
  "",     "",       "LB",   "LH",   "LWL",
  "LW",   "LBU",    "LHU",  "LWR",  "",
  "SB",   "SH",     "SWL",  "SW",  "",
  "",     "SWR",    "",     "LWC0", "LWC1",
  "LWC2", "LWC3",   "",     "",     "",
  "",     "SWC0",   "SWC1", "SWC2", "SWC3",
  "",     "",       "",     ""
];

const SECONDARY_OPS: [&str; 64] = [
  "SLL",   "",     "SRL",     "SRA",   "SLLV",
  "",      "SRLV", "SRAV",    "JR",    "JALR",
  "",      "",     "SYSCALL", "BREAK", "",
  "",      "MFHI", "MTHI",    "MFLO",  "MTLO",
  "",      "",     "",        "",      "MULT",
  "MULTU", "DIV",  "DIVU",    "",      "",
  "",      "",     "ADD",     "ADDU",  "SUB",
  "SUBU",  "AND",  "OR",      "XOR",   "NOR",
  "",      "",     "SLT",     "SLTU",  "",
  "",      "",     "",        "",      "",
  "",      "",     "",        "",      "",
  "",      "",     "",        "",      "",
  "",      "",     "",        "",
];

const PRIMARY_HANDLERS: [fn(&mut CPU, Instruction); 64] = [
  // 0x0
  CPU::secondary, CPU::bcondz,  CPU::j,       CPU::jal,     CPU::beq,
  CPU::bne,       CPU::blez,    CPU::bgtz,    CPU::addi,    CPU::addiu,
  // 0xa
  CPU::slti,      CPU::sltiu,   CPU::andi,    CPU::ori,     CPU::xori,
  CPU::lui,       CPU::cop0,    CPU::cop1,    CPU::cop2,    CPU::cop3,
  // 0x14
  CPU::illegal,   CPU::illegal, CPU::illegal, CPU::illegal, CPU::illegal,
  CPU::illegal,   CPU::illegal, CPU::illegal, CPU::illegal, CPU::illegal,
  // 0x1e
  CPU::illegal,   CPU::illegal, CPU::lb,      CPU::lh,      CPU::lwl,
  CPU::lw,        CPU::lbu,     CPU::lhu,     CPU::lwr,     CPU::illegal,
  // 0x28
  CPU::sb,        CPU::sh,      CPU::swl,     CPU::sw,     CPU::illegal,
  CPU::illegal,   CPU::swr,     CPU::illegal, CPU::lwc0,    CPU::lwc1,
  // 0x32
  CPU::lwc2,      CPU::lwc3,    CPU::illegal, CPU::illegal, CPU::illegal,
  CPU::illegal,   CPU::swc0,    CPU::swc1,    CPU::swc2,    CPU::swc3,
  // 0x3c
  CPU::illegal,   CPU::illegal, CPU::illegal, CPU::illegal
];

const SECONDARY_HANDLERS: [fn(&mut CPU, Instruction); 64] = [
  // 0x0
  CPU::sll,     CPU::illegal, CPU::srl,     CPU::sra,      CPU::sllv,
  CPU::illegal, CPU::srlv,    CPU::srav,    CPU::jr,       CPU::jalr,
  // 0xa
  CPU::illegal, CPU::illegal, CPU::syscall, CPU::op_break, CPU::illegal,
  CPU::illegal, CPU::mfhi,    CPU::mthi,    CPU::mflo,     CPU::mtlo,
  // 0x14
  CPU::illegal, CPU::illegal, CPU::illegal, CPU::illegal,  CPU::mult,
  CPU::multu,   CPU::div,     CPU::divu,    CPU::illegal,  CPU::illegal,
  // 0x1e
  CPU::illegal, CPU::illegal, CPU::add,     CPU::addu,     CPU::sub,
  CPU::subu,    CPU::and,     CPU::or,      CPU::xor,      CPU::nor,
  // 0x28
  CPU::illegal, CPU::illegal, CPU::slt,     CPU::sltu,     CPU::illegal,
  CPU::illegal, CPU::illegal, CPU::illegal, CPU::illegal,  CPU::illegal,
  // 0x32
  CPU::illegal, CPU::illegal, CPU::illegal, CPU::illegal,  CPU::illegal,
  CPU::illegal, CPU::illegal, CPU::illegal, CPU::illegal,  CPU::illegal,
  // 0x3c
  CPU::illegal, CPU::illegal, CPU::illegal, CPU::illegal
];

impl CPU {
  pub fn execute(&mut self, instr: Instruction) {
    let op_code = instr.op_code();

    if op_code != 0 && self.debug_on {
      println!("op: {}", PRIMARY_OPS[op_code as usize]);
    }

    let handler_fn = PRIMARY_HANDLERS[op_code as usize];

    handler_fn(self, instr);
  }

  fn bcondz(&mut self, instr: Instruction) {
    match instr.bcond() {
      0 => self.bltz(instr),
      1 => self.bgez(instr),
      _ => unreachable!("can't happen")
    }
  }

  fn secondary(&mut self, instr: Instruction) {
    let op_code = instr.op_code_secondary();

    if self.debug_on {
      println!("op: {}", SECONDARY_OPS[op_code as usize]);
    }

    let handler_fn = SECONDARY_HANDLERS[op_code as usize];

    handler_fn(self, instr);
  }

  fn illegal(&mut self, _instr: Instruction) {
    // panic!("illegal instruction received: {:02x}", instr.op_code());
    self.execute_load_delay();
    self.exception(Cause::IllegalInstruction);
  }

  fn sh(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let value = self.r[instr.rt()];

    self.execute_load_delay();

    if address & 0b1 == 0 {
      self.store_16(address, value as u16);
    } else {
      self.exception(Cause::StoreAddressError);
    }
  }

  fn sb(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let value = self.r[instr.rt()];

    self.execute_load_delay();

    self.store_8(address, value as u8);
  }

  fn lb(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let value = self.load_8(address);

    self.update_load(instr.rt(), value as i8 as i32 as u32);
  }

  fn lbu(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let value = self.load_8(address);

    self.update_load(instr.rt(), value as u32);
  }

  fn lhu(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    if address & 0b1 == 0 {
      let value = self.load_16(address);
      self.update_load(instr.rt(), value as u32);
    } else {
      self.execute_load_delay();
      self.exception(Cause::LoadAddressError);
    }
  }

  fn lh(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    if address & 0b1 == 0 {
      let value = self.load_16(address);

      self.update_load(instr.rt(), value as i16 as u32);
    } else {
      self.execute_load_delay();
      self.exception(Cause::LoadAddressError);
    }
  }

  fn cop0(&mut self, instr: Instruction) {
    let op_code = instr.cop_code();

    match op_code {
      0b00000 => self.mfc0(instr),
      0b00100 => self.mtc0(instr),
      0b10000 => self.rfe(instr),
      _ => panic!("cop0 instruction not implemented yet")
    }
  }

  fn cop1(&mut self, _: Instruction) {
    self.execute_load_delay();
    self.exception(Cause::CoprocessorError);
  }

  fn cop2(&mut self, instr: Instruction) {
    let op_code = instr.cop_code();

    match op_code {
      0b00000 => self.mfc2(instr),
      0b00010 => self.cfc2(instr),
      0b00100 => self.mtc2(instr),
      0b00110 => self.ctc2(instr),
      _ => if op_code & 0x10 == 0x10 {
        self.cop2_command(instr);
      } else {
        panic!("unknown instruction received: {:X}", op_code)
      }
    }
  }

  fn cfc2(&mut self, instr: Instruction) {
    let value = self.gte.read_control(instr.rd());

    self.update_load(instr.rt(), value);
  }

  fn mfc2(&mut self, instr: Instruction) {
    let value = self.gte.read_data(instr.rd());

    self.update_load(instr.rt(), value);
  }

  fn mtc2(&mut self, instr: Instruction) {
    let value = self.r[instr.rt()];

    self.gte.write_data(instr.rd(), value);

    self.execute_load_delay();
  }

  fn cop2_command(&mut self, instr: Instruction) {
    self.gte.execute_command(instr);

    self.execute_load_delay();
  }

  fn ctc2(&mut self, instr: Instruction) {
    let value = self.r[instr.rt()];

    self.gte.write_control(instr.rd(), value);

    self.execute_load_delay();
  }

  fn cop3(&mut self, _: Instruction) {
    self.execute_load_delay();
    self.exception(Cause::CoprocessorError);
  }

  fn lwc0(&mut self, _instr: Instruction) {
    self.execute_load_delay();

    self.exception(Cause::CoprocessorError);
  }

  fn lwc1(&mut self, _instr: Instruction) {
    self.execute_load_delay();
    self.exception(Cause::CoprocessorError);
  }

  fn lwc2(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    self.execute_load_delay();

    if address & 0b11 == 0 {
      let value = self.load_32(address);

      self.gte.write_data(instr.rt(), value);
    } else {
      self.exception(Cause::LoadAddressError);
    }
  }

  fn lwc3(&mut self, _instr: Instruction) {
    self.execute_load_delay();
    self.exception(Cause::CoprocessorError);
  }

  fn swc0(&mut self, _instr: Instruction) {
    self.execute_load_delay();
    self.exception(Cause::CoprocessorError);
  }

  fn swc1(&mut self, _instr: Instruction) {
    self.execute_load_delay();
    self.exception(Cause::CoprocessorError);
  }

  fn swc2(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let value = self.gte.read_data(instr.rt());

    self.execute_load_delay();

    self.store_32(address, value);
  }

  fn swc3(&mut self, _instr: Instruction) {
    self.execute_load_delay();
    self.exception(Cause::CoprocessorError);
  }

  fn bne(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, self.r[instr.rs()] != self.r[instr.rt()], true);
  }

  fn bltz(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.execute_load_delay();

    if instr.should_link() {
      self.set_reg(RA_REGISTER, self.next_pc);
    }

    self.branch_if(offset, (self.r[instr.rs()] as i32) < 0, false);
  }

  fn bgez(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.execute_load_delay();

    if instr.should_link() {
      self.set_reg(RA_REGISTER, self.next_pc);
    }

    self.branch_if(offset, (self.r[instr.rs()] as i32) >= 0, false);
  }

  fn bgtz(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, (self.r[instr.rs()] as i32) > 0, true);
  }

  fn blez(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, (self.r[instr.rs()] as i32) <= 0, true);
  }

  fn beq(&mut self, instr: Instruction) {
    let offset = instr.immediate_signed();

    self.branch_if(offset, self.r[instr.rs()] == self.r[instr.rt()], true);
  }

  fn branch_if(&mut self, offset: u32, condition: bool, should_execute_load_delay: bool) {
    if condition {
      self.branch(offset);
    }

    if should_execute_load_delay {
      self.execute_load_delay();
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

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn slt(&mut self, instr: Instruction) {
    let result: u32 = if (self.r[instr.rs()] as i32) < (self.r[instr.rt()] as i32) {
      1
    } else {
      0
    };

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn sltiu(&mut self, instr: Instruction) {
    let result: u32 = if self.r[instr.rs()] < instr.immediate_signed() {
      1
    } else {
      0
    };

    self.execute_load_delay();

    self.set_reg(instr.rt(), result);
  }

  fn slti(&mut self, instr: Instruction) {
    let result: u32 = if (self.r[instr.rs()] as i32) < (instr.immediate_signed() as i32) {
      1
    } else {
      0
    };

    self.execute_load_delay();

    self.set_reg(instr.rt(), result);
  }

  fn mfc0(&mut self, instr: Instruction) {
    let delayed_register = instr.rt();
    let delayed_load = match instr.rd() {
      12 => self.cop0.sr,
      13 => self.cop0.cause,
      14 => self.cop0.epc,
      15 => 0x0000_0002,
      _ => panic!("unhandled read from cop0 register: {}", instr.rd())
    };

    self.update_load(delayed_register, delayed_load);
  }

  fn mtc0(&mut self, instr: Instruction) {

    let cop0_reg = instr.rd();
    let value = self.r[instr.rt()];

    self.execute_load_delay();

    match cop0_reg {
      3 | 5 | 6 | 7 | 9 | 11 => {
        if value != 0 {
          panic!("unhandled write to debug registers");
        }
      }
      12 => self.cop0.sr = value,
      13 => self.cop0.cause = value,
      _ => panic!("cop0 register not implemented in mtc0: {}", cop0_reg)
    }
  }

  fn lw(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    if address & 0b11 == 0 {
      let val = self.load_32(address);
      self.update_load(instr.rt(), val);
    } else {
      self.execute_load_delay();
      self.exception(Cause::LoadAddressError);
    }
}

  fn lwl(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let mut result = self.r[instr.rt()];

    if let Some((reg, val)) = self.load {
      if reg == instr.rt() {
        result = val;
      }
    }

    let aligned_address = address & !0x3;
    let aligned_word = self.load_32(aligned_address);

    result = match address & 0x3 {
      0 => (result & 0xffffff) | (aligned_word << 24),
      1 => (result & 0xffff) | (aligned_word << 16),
      2 => (result & 0xff) | (aligned_word << 8),
      3 => aligned_word,
      _ => unreachable!("can't happen")
    };

    self.update_load(instr.rt(), result);
  }

  fn lwr(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let mut result = self.r[instr.rt()];

    if let Some((reg, val)) = self.load {
      if reg == instr.rt() {
        result = val;
      }
    }

    let aligned_address = address & !0x3;
    let aligned_word = self.load_32(aligned_address);

    result = match address & 0x3 {
      0 => aligned_word,
      1 => (result & 0xff000000) | (aligned_word >> 8),
      2 => (result & 0xffff0000) | (aligned_word >> 16),
      3 => (result & 0xffffff00) | (aligned_word >> 24),
      _ => unreachable!("can't happen")
    };

    self.update_load(instr.rt(), result);
  }

  fn j(&mut self, instr: Instruction) {
    self.execute_load_delay();

    self.next_pc = (self.pc & 0xf000_0000) | (instr.j_imm() << 2);

    self.branch = true;
  }

  fn jal(&mut self, instr: Instruction) {
    let ra = self.next_pc;
    self.j(instr);

    self.set_reg(RA_REGISTER, ra);
  }

  fn jalr(&mut self, instr: Instruction) {
    let ra = self.next_pc;

    self.next_pc = self.r[instr.rs()];
    self.branch = true;

    self.execute_load_delay();

    self.set_reg(instr.rd(), ra);
  }

  fn jr(&mut self, instr: Instruction) {
    self.next_pc = self.r[instr.rs()];
    self.branch = true;

    self.execute_load_delay();
  }

  fn sll(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] << instr.imm5();

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn sllv(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] << (self.r[instr.rs()] & 0x1f);

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn srlv(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] >> (self.r[instr.rs()] & 0x1f);

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn sra(&mut self, instr: Instruction) {
    let result = (self.r[instr.rt()] as i32) >> instr.imm5();

    self.execute_load_delay();

    self.set_reg(instr.rd(), result as u32);
  }

  fn srav(&mut self, instr: Instruction) {
    let result = (self.r[instr.rt()] as i32) >> (self.r[instr.rs()] & 0x1f);

    self.execute_load_delay();

    self.set_reg(instr.rd(), result as u32);
  }

  fn srl(&mut self, instr: Instruction) {
    let result = self.r[instr.rt()] >> instr.imm5();

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn lui(&mut self, instr: Instruction) {
    let result = instr.immediate() << 16;

    self.execute_load_delay();

    self.set_reg(instr.rt(), result);
  }

  fn mflo(&mut self, instr: Instruction) {
    let result = self.low;

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn mtlo(&mut self, instr: Instruction) {
    self.low = self.r[instr.rs()];

    self.execute_load_delay();
  }

  fn mfhi(&mut self, instr: Instruction) {
    let result = self.hi;

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn mthi(&mut self, instr: Instruction) {
    self.hi = self.r[instr.rs()];

    self.execute_load_delay();
  }

  fn ori(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] | instr.immediate();

    self.execute_load_delay();

    self.set_reg(instr.rt(), result);
  }

  fn or(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] | self.r[instr.rt()];

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn xori(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] ^ instr.immediate();

    self.execute_load_delay();

    self.set_reg(instr.rt(), result);
  }

  fn xor(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] ^ self.r[instr.rt()];

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn nor(&mut self, instr: Instruction) {
    let result = !(self.r[instr.rs()] | self.r[instr.rt()]);

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn andi(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] & instr.immediate();

    self.execute_load_delay();

    self.set_reg(instr.rt(), result);
  }

  fn and(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()] & self.r[instr.rt()];

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn addi(&mut self, instr: Instruction) {

    let rs = self.r[instr.rs()] as i32;

    self.execute_load_delay();

    if let Some(result) = rs.checked_add(instr.immediate_signed() as i32) {
      self.set_reg(instr.rt(), result as u32);
    } else {
      self.exception(Cause::Overflow);
    }
  }

  fn add(&mut self, instr: Instruction) {
    let rs = self.r[instr.rs()] as i32;

    self.execute_load_delay();

    if let Some(result) = rs.checked_add(self.r[instr.rt()] as i32) {
      self.set_reg(instr.rd(), result as u32);
    } else {
      self.exception(Cause::Overflow);
    }
  }

  fn addiu(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    self.execute_load_delay();

    self.set_reg(instr.rt(), result);
  }

  fn addu(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()].wrapping_add(self.r[instr.rt()]);

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn subu(&mut self, instr: Instruction) {
    let result = self.r[instr.rs()].wrapping_sub(self.r[instr.rt()]);

    self.execute_load_delay();

    self.set_reg(instr.rd(), result);
  }

  fn sub(&mut self, instr: Instruction) {
    let rs = self.r[instr.rs()] as i32;

    self.execute_load_delay();

    if let Some(result) = rs.checked_sub(self.r[instr.rt()] as i32) {
      self.set_reg(instr.rd(), result as u32);
    } else {
      self.exception(Cause::Overflow);
    }
  }

  fn multu(&mut self, instr: Instruction) {
    let a = self.r[instr.rs()] as u64;
    let b = self.r[instr.rt()] as u64;

    let result = a * b;

    self.execute_load_delay();

    self.low = result as u32;
    self.hi = (result >> 32) as u32;
  }

  fn mult(&mut self, instr: Instruction) {
    let a = self.r[instr.rs()] as i32 as i64;
    let b = self.r[instr.rt()] as i32 as i64;

    let result = a * b;

    self.execute_load_delay();

    self.low = result as u32;
    self.hi = (result >> 32) as u32;
  }

  fn div(&mut self, instr: Instruction) {
    let numerator = self.r[instr.rs()] as i32;
    let denominator = self.r[instr.rt()] as i32;

    self.execute_load_delay();

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

    self.execute_load_delay();

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
    let mem_value = self.load_32(aligned_address);

    let result = match address & 0x3 {
      0 => (mem_value & 0xffffff00) | (val >> 24),
      1 => (mem_value & 0xffff0000) | (val >> 16),
      2 => (mem_value & 0xff000000) | (val >> 8),
      3 => val,
      _ => unreachable!("can't happen")
    };

    self.execute_load_delay();

    self.store_32(aligned_address, result);
  }

  fn swr(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let aligned_address = address & !0x3;

    let val = self.r[instr.rt()];
    let mem_value = self.load_32(aligned_address);

    let result = match address & 0x3 {
      0 => val,
      1 => (mem_value & 0xff) | (val << 8),
      2 => (mem_value & 0xffff) | (val << 16),
      3 => (mem_value & 0xffffff) | (val << 24),
      _ => unreachable!("can't happen")
    };

    self.execute_load_delay();

    self.store_32(aligned_address, result);
  }

  fn sw(&mut self, instr: Instruction) {
    let address = self.r[instr.rs()].wrapping_add(instr.immediate_signed());

    let value = self.r[instr.rt()];

    self.execute_load_delay();

    if address & 0b11 == 0 {
      self.store_32(address, value);
    } else {
      self.exception(Cause::StoreAddressError);
    }
  }

  fn syscall(&mut self, _instr: Instruction) {
    self.execute_load_delay();

    self.exception(Cause::SysCall);
  }

  fn op_break(&mut self, _instr: Instruction) {
    self.execute_load_delay();

    self.exception(Cause::Break);
  }

  fn rfe(&mut self, instr: Instruction) {
    self.execute_load_delay();

    if instr.cop0_lower_bits() != 0b010000 {
      panic!("illegal cop0 instruction received")
    }
    self.cop0.return_from_exception();
  }

  fn update_load(&mut self, reg: usize, val: u32) {
    if let Some((pending_reg, _)) = self.load {
      if reg != pending_reg {
        self.execute_load_delay();
      }
    }

    self.load = Some((reg, val));
  }

  pub fn execute_load_delay(&mut self) {
    if let Some((reg, value)) = self.load {
      self.set_reg(reg, value);
    }

    self.load = None;
  }
}