use std::fs;

use rustation::cpu::CPU;

extern crate rustation;

pub fn main() {
  // let args: Vec<String> = env::args().collect();

  // if args.len() != 2 {
  //   panic!("please specify a file");
  // }

  // let filepath = &args[1];

  // let bytes: Vec<u8> = fs::read(filepath).unwrap();

  let mut cpu = CPU::new(fs::read("../SCPH1001.BIN").unwrap());

  loop {
    cpu.step();
  }
}