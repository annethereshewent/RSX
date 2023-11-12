use std::{rc::Rc, cell::Cell, time::{UNIX_EPOCH, SystemTime, Duration}, thread::sleep};

use crate::cpu::{CPU_FREQUENCY, interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt}, timers::timers::Timers};

use self::gpu_stat_register::{GpuStatRegister, VideoMode, TextureColors};

pub mod gpu_stat_register;
pub mod render;

const COMMAND_LENGTH: [u32; 256] = [
  1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  4, 4, 4, 4, 7, 7, 7, 7, 5, 5, 5, 5, 9, 9, 9, 9, 6, 6, 6, 6, 9, 9, 9, 9, 8, 8, 8, 8, 12, 12, 12,
  12, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
  4, 4, 3, 1, 3, 1, 4, 4, 4, 4, 2, 1, 2, 1, 3, 3, 3, 3, 2, 1, 2, 1, 3, 3, 3, 3, 2, 1, 2, 1, 3, 3,
  3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
  4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
  3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
  3, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  1, 1,
];

pub const CYCLES_PER_SCANLINE: usize = 3413;
pub const NUM_SCANLINES_PER_FRAME: usize = 263;

pub const GPU_FREQUENCY: f64 = 53_693_181.818;
pub const GPU_CYCLES_TO_CPU_CYCLES: f64 = GPU_FREQUENCY / CPU_FREQUENCY;

pub const CYCLES_IN_HSYNC: i32 = 200;

pub const FPS_INTERVAL: u128 = 1000 / 60;


#[derive(Clone, Copy)]
struct TextureCache {
  tag: isize,
  data: [u8; 8]
}


impl TextureCache {
  pub fn new() -> Self {
    Self {
      tag: -1,
      data: [0; 8]
    }
  }
}

struct ImageTransfer {
  pub x: u32,
  pub y: u32,
  pub w: u32,
  pub h: u32,
  pub transferred_x: u32,
  pub transferred_y: u32,
  pub is_active: bool
}

impl ImageTransfer {
  pub fn new() -> Self {
    Self {
      x: 0,
      y: 0,
      transferred_x: 0,
      transferred_y: 0,
      w: 0,
      h: 0,
      is_active: false
    }
  }
}

pub struct GPU {
  pub stat: GpuStatRegister,
  texture_rectangle_x_flip: bool,
  texture_rectangle_y_flip: bool,
  texture_window_x_mask: u8,
  texture_window_y_mask: u8,
  texture_window_x_offset: u8,
  texture_window_y_offset: u8,
  drawing_area_top: u16,
  drawing_area_left: u16,
  drawing_area_right: u16,
  drawing_area_bottom: u16,
  drawing_x_offset: i16,
  drawing_y_offset: i16,
  drawing_vram_x_start: u16,
  drawing_vram_y_start: u16,
  display_horizontal_start: u16,
  display_horizontal_end: u16,
  display_line_start: u16,
  display_line_end: u16,
  display_vram_x_start: u16,
  display_vram_y_start: u16,
  command_buffer: [u32; 12],
  command_index: usize,
  words_remaining: u32,
  // halfwords_remaining: u32,
  cycles: i32,
  dotclock_cycles: i32,
  num_scanlines: u32,
  current_scanline: u32,
  pub frame_complete: bool,
  interrupts: Rc<Cell<InterruptRegisters>>,
  previous_time: u128,
  image_transfer: ImageTransfer,
  vram: Box<[u8]>,
  pub picture: Box<[u8]>,
  texture_cache: [TextureCache; 256],
  clut_tag: isize,
  clut_cache: [u16; 256],
  current_texture_x_base: u8,
  current_texture_y_base: u8,
  current_clut: (i32, i32),
  current_texture_colors: TextureColors,

}

impl GPU {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    Self {
      stat: GpuStatRegister::new(),
      texture_rectangle_x_flip: false,
      texture_rectangle_y_flip: false,
      texture_window_x_mask: 0,
      texture_window_y_mask: 0,
      texture_window_y_offset: 0,
      drawing_area_top: 0,
      drawing_area_left: 0,
      drawing_area_right: 0,
      drawing_area_bottom: 0,
      drawing_x_offset: 0,
      drawing_y_offset: 0,
      drawing_vram_x_start: 0,
      drawing_vram_y_start: 0,
      display_horizontal_start: 512,
      display_horizontal_end: 3072,
      display_line_start: 16,
      display_line_end: 256,
      texture_window_x_offset: 0,
      display_vram_x_start: 0,
      display_vram_y_start: 0,
      command_buffer: [0; 12],
      command_index: 0,
      words_remaining: 0,
      // halfwords_remaining: 0,
      cycles: 0,
      num_scanlines: 263,
      current_scanline: 0,
      frame_complete: false,
      interrupts,
      dotclock_cycles: 0,
      previous_time: 0,
      image_transfer: ImageTransfer::new(),
      vram: vec![0; 0x100000].into_boxed_slice(),
      picture: vec![0; 1024 * 512 * 3].into_boxed_slice(),
      texture_cache: [TextureCache::new(); 256],
      clut_tag: -1,
      clut_cache: [0; 256],
      current_clut: (0,0),
      current_texture_colors: TextureColors::FourBit,
      current_texture_x_base: 0,
      current_texture_y_base: 0
    }
  }

  pub fn cap_fps(&mut self) {
    let current_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("an error occurred")
      .as_millis();

    if self.previous_time != 0 {
      let diff = current_time - self.previous_time;
      if diff < FPS_INTERVAL {
        sleep(Duration::from_millis((FPS_INTERVAL - diff) as u64));
      }
    }

    self.previous_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("an error occurred")
      .as_millis();
  }

  pub fn tick(&mut self, cycles: i32, timers: &mut Timers) {
    let elapsed_gpu_cycles = ((cycles as f64) * GPU_CYCLES_TO_CPU_CYCLES).round() as i32;

    let dotclock = self.get_dotclock();

    let previous_hblank = self.in_hblank();

    self.cycles += elapsed_gpu_cycles;
    self.dotclock_cycles += elapsed_gpu_cycles;

    timers.tick_dotclock(elapsed_gpu_cycles / dotclock);

    self.dotclock_cycles %= dotclock;

    let horizontal_cycles = match self.stat.video_mode {
      VideoMode::Ntsc => 3413,
      VideoMode::Pal => 3406
    };

    if self.cycles >= horizontal_cycles {
      self.cycles -= horizontal_cycles;

      if self.stat.vertical_resolution == 240 && self.stat.vertical_interlace {
        self.stat.even_odd = !self.stat.even_odd;
      }

      self.current_scanline += 1;

      if self.current_scanline == (self.num_scanlines - 20) {
        self.frame_complete = true;
        self.cap_fps();
        // entering VBlank
        let mut interrupts = self.interrupts.get();

        interrupts.status.set_interrupt(Interrupt::Vblank);
        timers.set_vblank(true);

        self.interrupts.set(interrupts);
      }

      if self.current_scanline == self.num_scanlines {
        // exiting vblank
        self.num_scanlines = if self.num_scanlines == 263 {
          262
        } else {
          263
        };

        if self.stat.vertical_resolution == 480 && self.stat.vertical_interlace {
          self.stat.even_odd = !self.stat.even_odd;
        }
        self.current_scanline = 0;
        timers.set_vblank(false);
      }
    }

    if self.in_hblank() {
      if !previous_hblank {
        timers.set_hblank(true);
      }
    } else {
      if previous_hblank {
        timers.set_hblank(false);
      }
    }

    if self.stat.irq_enabled {
      let mut interrupts = self.interrupts.get();

      interrupts.status.set_interrupt(Interrupt::Gpu);

      self.interrupts.set(interrupts);
    }
  }

  fn transfer_to_vram(&mut self, val: u16) {
    let curr_x = self.image_transfer.x + self.image_transfer.transferred_x;
    let curr_y = self.image_transfer.y + self.image_transfer.transferred_y;

    self.image_transfer.transferred_x += 1;

    let vram_address = self.get_vram_address(curr_x & 0x3ff, curr_y & 0x1ff);

    if self.image_transfer.transferred_x == self.image_transfer.w {
      self.image_transfer.transferred_x = 0;

      self.image_transfer.transferred_y += 1;

      if self.image_transfer.transferred_y == self.image_transfer.h {
        self.image_transfer.is_active = false;
      }
    }

    self.vram[vram_address] = val as u8;
    self.vram[vram_address + 1] = (val >> 8) as u8;
  }

  pub fn get_vram_address(&mut self, x: u32, y: u32) -> usize {
    2 * (((x & 0x3ff) + 1024 * (y & 0x1ff))) as usize
  }

  pub fn get_vram_address_24(&mut self, x: u32, y: u32) -> usize {
    3 * (((x & 0x3ff) + 2048 * (y & 0x1ff))) as usize
  }

  pub fn in_hblank(&self) -> bool {
    self.cycles < self.display_horizontal_start as i32
      || self.cycles >= self.display_horizontal_end as i32
  }

  pub fn get_dotclock(&self) -> i32 {
    match self.stat.horizontal_resolution {
      320 => 8,
      640 => 4,
      256 => 10,
      512 => 5,
      368 => 7,
      _ => unreachable!(),
    }
  }

  pub fn stat_value(&self) -> u32 {
    let interlace_line = if self.current_scanline >= (self.num_scanlines - 20) {
      false
    } else {
      self.stat.even_odd
    };

    self.stat.value(interlace_line)
  }

  pub fn gp0(&mut self, val: u32) {
    if self.image_transfer.is_active {
      self.transfer_to_vram(val as u16);

      if self.image_transfer.is_active {
        self.transfer_to_vram((val >> 16) as u16);
      }

      return;
    }

    self.command_buffer[self.command_index] = val;
    self.command_index += 1;

    if self.words_remaining == 0 {
      let op_code = val >> 24;
      self.words_remaining = COMMAND_LENGTH[op_code as usize];
    }

    if self.words_remaining == 1 {
      // execute the command
      self.execute_gp0();
      self.command_index = 0;
    }

    self.words_remaining -= 1;
  }

  fn execute_gp0(&mut self) {
    let command = self.command_buffer[0];

    let op_code = command >> 24;

    match op_code {
      0x00 => (), // NOP,
      0x01 => self.gp0_invalidate_cache(),
      0x28 => self.gp0_monochrome_quadrilateral(),
      0x2c => self.gp0_textured_quad_with_blending(),
      0x30 => self.gp0_shaded_triangle(),
      0x38 => self.gp0_shaded_quadrilateral(),
      0xa0 => self.gp0_image_transfer_to_vram(),
      0xc0 => self.gp0_image_transfer_to_cpu(),
      0xe1 => self.gp0_draw_mode(),
      0xe2 => self.gp0_texture_window(),
      0xe3 => self.gp0_draw_area_top_left(),
      0xe4 => self.gp0_draw_area_bottom_right(),
      0xe5 => self.gp0_drawing_offset(),
      0xe6 => self.gp0_mask_bit(),
      _ => todo!("invalid or unsupported GP0 command: {:02x}", op_code)
    }
  }

  pub fn gp1(&mut self, val: u32) {
    let op_code = val >> 24;

    match op_code {
      0x00 => self.gp1_reset(val),
      0x01 => self.gp1_clear_command_buffer(),
      0x02 => self.gp1_acknowledge_interrupt(),
      0x03 => self.gp1_display_enable(val),
      0x04 => self.gp1_dma_dir(val),
      0x05 => self.gp1_display_vram_start(val),
      0x06 => self.gp1_display_horizontal_range(val),
      0x07 => self.gp1_display_vertical_range(val),
      0x08 => self.gp1_display_mode(val),
      _ => todo!("Invalid or unsupported GP1 command: {:02x}", op_code)
    }
  }

  fn gp1_clear_command_buffer(&mut self) {
    self.command_index = 0;
    self.words_remaining = 0;
  }

  fn gp1_acknowledge_interrupt(&mut self) {
    self.stat.irq_enabled = false;
  }

  fn gp0_image_transfer_to_cpu(&mut self) {
    let dimensions = self.command_buffer[2];

    let width = dimensions as u16;
    let height = (dimensions >> 16) as u16;

    // TODO: do something with this data
  }

  pub fn parse_color(val: u32) -> (u8,u8,u8) {
    let r = val as u8;
    let g = (val >> 8) as u8;
    let b = (val >> 16) as u8;

    (r, g, b)
  }

  pub fn parse_position(&self, val: u32) -> (i32, i32) {
    let mut x = (val & 0xffff) as i32;
    let mut y = (val >> 16) as i32;

    x = GPU::sign_extend_i32(x, 11);
    y = GPU::sign_extend_i32(y, 11);

    let x_offset = self.drawing_x_offset as i32;
    let y_offset = self.drawing_y_offset as i32;

    (x + x_offset, y + y_offset)
  }

  fn sign_extend_i32(mut value: i32, size: usize) -> i32 {
    let sign = 1 << (size - 1);
    let mask = (1 << size) - 1;

    if (value & sign) != 0 {
        value |= !mask;
    } else {
        value &= mask;
    }

    return value;
}

  fn gp0_shaded_quadrilateral(&mut self) {
    let mut colors = [
      GPU::parse_color(self.command_buffer[0]),
      GPU::parse_color(self.command_buffer[2]),
      GPU::parse_color(self.command_buffer[4]),
      GPU::parse_color(self.command_buffer[6])
    ];

    let mut positions = [
      self.parse_position(self.command_buffer[1]),
      self.parse_position(self.command_buffer[3]),
      self.parse_position(self.command_buffer[5]),
      self.parse_position(self.command_buffer[7])
    ];

    self.rasterize_triangle(&mut colors[0..3], &mut positions[0..3], None, None, true, false);
    self.rasterize_triangle(&mut colors[1..4], &mut positions[1..4], None, None, true, false);
  }

  fn gp0_shaded_triangle(&mut self) {
    let mut colors = [
      GPU::parse_color(self.command_buffer[0]),
      GPU::parse_color(self.command_buffer[2]),
      GPU::parse_color(self.command_buffer[4])
    ];

    let mut positions = [
      self.parse_position(self.command_buffer[1]),
      self.parse_position(self.command_buffer[3]),
      self.parse_position(self.command_buffer[5])
    ];

    self.rasterize_triangle(&mut colors[0..3], &mut positions[0..3], None, None, true, false);
  }

  fn gp0_textured_quad_with_blending(&mut self) {
    let color = GPU::parse_color(self.command_buffer[0]);
    let mut colors = [
      color,
      color,
      color,
      color
    ];

    let mut positions = [
      self.parse_position(self.command_buffer[1]),
      self.parse_position(self.command_buffer[3]),
      self.parse_position(self.command_buffer[5]),
      self.parse_position(self.command_buffer[7])
    ];

    let mut texture_positions = [
      GPU::parse_texture_coords(self.command_buffer[2]),
      GPU::parse_texture_coords(self.command_buffer[4]),
      GPU::parse_texture_coords(self.command_buffer[6]),
      GPU::parse_texture_coords(self.command_buffer[8]),
    ];

    let clut = GPU::to_clut(self.command_buffer[2]);
    self.parse_texture_data(self.command_buffer[4]);

    if clut != self.current_clut ||
      self.stat.texture_x_base != self.current_texture_x_base ||
      self.stat.texture_y_base1 != self.current_texture_y_base ||
      self.stat.texture_colors != self.current_texture_colors {
        self.gp0_invalidate_cache();
    }

    self.current_texture_x_base = self.stat.texture_x_base;
    self.current_texture_y_base = self.stat.texture_y_base1;
    self.current_texture_colors = self.stat.texture_colors;
    self.current_clut = clut;

    self.rasterize_triangle(&mut colors[0..3], &mut positions[0..3], Some(&mut texture_positions[0..3]), Some(clut), false, true);
    self.rasterize_triangle(&mut colors[1..4], &mut positions[1..4], Some(&mut texture_positions[1..4]), Some(clut), false, true);

  }

  fn parse_texture_coords(command: u32) -> (i32, i32) {
    let x = (command & 0xff) as i32;
    let y = ((command >> 8) & 0xff) as i32;

    (x, y)
  }

  fn parse_texture_data(&mut self, command: u32) {
    let texture_data = command >> 16;

    self.texture_rectangle_x_flip = (texture_data >> 13) & 0b1 == 1;
    self.texture_rectangle_y_flip = (texture_data >> 12) & 0b1 == 1;
    self.stat.texture_y_base2 = ((texture_data >> 11) & 0b1) as u8;
    self.stat.draw_to_display = (texture_data >> 10) & 0b1 == 1;
    self.stat.dither_enabled = (texture_data >> 9) & 0b1 == 1;

    self.stat.texture_colors = match (texture_data >> 7) & 0b11 {
      0 => TextureColors::FourBit,
      1 => TextureColors::EightBit,
      2 => TextureColors::FifteenBit,
      n => panic!("invalid value received: {n}")
    };

    self.stat.semi_transparency = ((texture_data >> 5) & 0b11) as u8;
    self.stat.texture_x_base = (texture_data & 0xf) as u8;
    self.stat.texture_y_base1 = ((texture_data >> 4) & 0b1) as u8;
  }

  fn to_clut(command: u32) -> (i32, i32) {
    let x = ((command >> 16) & 0x3f) << 4;
    let y = ((command >> 16) & 0x7fc0) >> 6;

    (x as i32, y as i32)
  }

  fn gp0_image_transfer_to_vram(&mut self) {
    let coordinates = self.command_buffer[1];
    let dimensions = self.command_buffer[2];

    let x = coordinates & 0x3ff;
    let y = (coordinates >> 16) & 0x3ff;

    let w = dimensions & 0x3ff;
    let h = (dimensions >> 16) & 0x1ff;

    self.image_transfer.x = x;
    self.image_transfer.y = y;

    self.image_transfer.transferred_x = 0;
    self.image_transfer.transferred_y = 0;

    self.image_transfer.w = if w > 0 { w } else { 0x400 };
    self.image_transfer.h = if h > 0 { h } else { 0x200 };

    self.image_transfer.is_active = true;
  }

  fn gp0_mask_bit(&mut self) {
    let val = self.command_buffer[0];

    self.stat.set_mask_attributes(val);
  }

  fn gp0_monochrome_quadrilateral(&mut self) {
    let color = GPU::parse_color(self.command_buffer[0]);

    let mut positions = [
      self.parse_position(self.command_buffer[1]),
      self.parse_position(self.command_buffer[2]),
      self.parse_position(self.command_buffer[3]),
      self.parse_position(self.command_buffer[4])
    ];

    self.rasterize_triangle(&mut [color, color, color][0..3], &mut positions[0..3], None, None, false, false);
    self.rasterize_triangle(&mut [color, color, color][0..3], &mut positions[1..4], None, None, false, false);
  }

  fn gp0_texture_window(&mut self) {
    let val = self.command_buffer[0];

    self.texture_window_x_mask = (val & 0x1f) as u8;
    self.texture_window_y_mask = ((val >> 5) & 0x1f) as u8;
    self.texture_window_x_offset = ((val >> 10) & 0x1f) as u8;
    self.texture_window_y_offset = ((val >> 15) & 0x1f) as u8;
  }

  fn gp0_draw_mode(&mut self) {
    let val = self.command_buffer[0];

    self.stat.update_draw_mode(val);

    self.texture_rectangle_x_flip = ((val >> 12) & 0b1) == 1;
    self.texture_rectangle_y_flip = ((val >> 13) & 0b1) == 1;
  }

  fn gp0_draw_area_top_left(&mut self) {
    let val = self.command_buffer[0];

    self.drawing_area_left = (val & 0x3ff) as u16;
    self.drawing_area_top = ((val >> 10) & 0x3ff) as u16;
  }

  fn gp0_draw_area_bottom_right(&mut self) {
    let val = self.command_buffer[0];

    self.drawing_area_right = (val & 0x3ff) as u16;
    self.drawing_area_bottom = ((val >> 10) & 0x3ff) as u16;
  }

  fn gp0_drawing_offset(&mut self) {
    let val = self.command_buffer[0];

    let x = (val & 0x7ff) as u16;
    let y = ((val >> 11) & 0x7ff) as u16;

    self.drawing_x_offset = ((x << 5) as i16) >> 5;
    self.drawing_y_offset = ((y << 5) as i16) >> 5;
  }

  fn gp0_invalidate_cache(&mut self) {
    for cache_entry in &mut self.texture_cache {
      cache_entry.tag = -1;
    }

    self.clut_tag = -1;
  }

  fn gp1_display_mode(&mut self, val: u32) {
    self.stat.update_display_mode(val);
  }

  fn gp1_display_enable(&mut self, val: u32) {
    self.stat.display_enable = val & 0b1 == 0;
  }

  fn gp1_display_vram_start(&mut self, val: u32) {
    self.display_vram_x_start = ((val & 0x3fe)) as u16;
    self.display_vram_y_start = ((val >> 10) & 0x1ff) as u16;
  }

  fn gp1_display_horizontal_range(&mut self, val: u32) {
    self.display_horizontal_start = (val & 0xfff) as u16;
    self.display_horizontal_end = ((val >> 12) & 0xfff) as u16;
  }

  fn gp1_display_vertical_range(&mut self, val: u32) {
    self.display_line_start = (val & 0x3ff) as u16;
    self.display_line_end = ((val >> 10) & 0x3ff) as u16;
  }

  fn gp1_dma_dir(&mut self, val: u32) {
    self.stat.update_dma_dir(val);
  }

  fn gp1_reset(&mut self, _: u32) {
    self.gp1_clear_command_buffer();
    self.stat.reset();

    self.texture_window_x_mask = 0;
    self.texture_window_y_mask = 0;

    self.texture_window_x_offset = 0;
    self.texture_window_y_offset = 0;

    self.texture_rectangle_x_flip = false;
    self.texture_rectangle_y_flip = false;

    self.drawing_area_left = 0;
    self.drawing_area_bottom = 0;
    self.drawing_area_right = 0;
    self.drawing_area_top = 0;

    self.drawing_x_offset = 0;
    self.drawing_y_offset = 0;

    self.display_vram_x_start = 0;
    self.display_vram_y_start = 0;

    self.display_horizontal_start = 0x200;
    self.display_horizontal_end = 0xc00;

    self.display_line_start = 0x10;
    self.display_line_end = 0x100;
  }
}