use std::{rc::Rc, cell::Cell, time::{UNIX_EPOCH, SystemTime, Duration}, thread::sleep};

use crate::{cpu::{CPU_FREQUENCY, interrupt::{interrupt_registers::InterruptRegisters, interrupt_register::Interrupt}, timers::timers::Timers}, util};

use self::gpu_stat_register::{GpuStatRegister, VideoMode, TextureColors, SemiTransparency};

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

// per https://psx-spx.consoledev.net/graphicsprocessingunitgpu/#24bit-rgb-to-15bit-rgb-dithering-enabled-in-texpage-attribute
const DITHER_OFFSETS: [[i16; 4]; 4] = [
  [-4, 0, -3, 1],
  [2, -2, 3, -1],
  [-3, 1, -4, 0],
  [3, -1, 2, -2],
];

pub const CYCLES_PER_SCANLINE: usize = 3413;
pub const NUM_SCANLINES_PER_FRAME: usize = 263;

pub const GPU_FREQUENCY: f64 = 53_693_181.818;
pub const GPU_CYCLES_TO_CPU_CYCLES: f64 = GPU_FREQUENCY / CPU_FREQUENCY;

pub const CYCLES_IN_HSYNC: i32 = 200;

pub const FPS_INTERVAL: u128 = 1000 / 60;

const VRAM_SIZE: usize = 2 * 1024 * 512;

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
  pub p: Coordinates2d,
  pub c: RgbColor,
  pub uv: Coordinates2d
}

impl Vertex {
  pub fn new(p: Coordinates2d, c: RgbColor, uv: Coordinates2d) -> Self {
    Self {
      p,
      uv,
      c
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct Coordinates2d {
  pub x: i32,
  pub y: i32
}

impl Coordinates2d {
  pub fn new(x: i32, y: i32) -> Self {
    Self {
      x,
      y
    }
  }
}

#[derive(Copy, Clone)]
pub struct Coordinates3d {
  pub x: i32,
  pub y: i32,
  pub z: i32
}

impl Coordinates3d {
  pub fn new(x: i32, y: i32, z: i32) -> Self {
    Self {
      x,
      y,
      z
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct RgbColor {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: bool
}

impl RgbColor {
  fn new(r: u8, g: u8, b: u8, a: bool) -> Self {
    Self {
      r,
      g,
      b,
      a
    }
  }
}

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

struct Transfer {
  pub x: u32,
  pub y: u32,
  pub w: u32,
  pub h: u32,
  pub read_x: u32,
  pub read_y: u32,
  pub is_active: bool
}

impl Transfer {
  pub fn new() -> Self {
    Self {
      x: 0,
      y: 0,
      read_x: 0,
      read_y: 0,
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
  cycles: i32,
  cpu_cycles: i32,
  dotclock_cycles: i32,
  num_scanlines: u32,
  current_scanline: u32,
  pub frame_complete: bool,
  interrupts: Rc<Cell<InterruptRegisters>>,
  previous_time: u128,
  image_transfer: Transfer,
  cpu_transfer: Transfer,
  vram: Box<[u8]>,
  pub picture: Box<[u8]>,
  texture_cache: [TextureCache; 256],
  clut_tag: isize,
  clut_cache: [u16; 256],
  current_texture_x_base: u8,
  current_texture_y_base: u8,
  current_clut: Coordinates2d,
  current_texture_colors: TextureColors,
  gpuread: u32,
  texture_window: u32,
  drawing_area_top_left: u32,
  drawing_area_bottom_right: u32,
  draw_offset: u32,
  pub debug_on: bool,
  dither_table: [[[u8; 0x200]; 4]; 4],
  polyline: bool,
  polyline_words_remaining: u8,
  polyline_prev_coord: Coordinates2d,
  polyline_prev_color: RgbColor,
  polyline_shaded: bool,
  polyline_semitransparent: bool
}

impl GPU {
  pub fn new(interrupts: Rc<Cell<InterruptRegisters>>) -> Self {
    let mut dither_table = [[[0; 0x200]; 4]; 4];

    for x in 0..4 {
      for y in 0..4 {
        for i in 0..0x200 {
          let out = i + DITHER_OFFSETS[x][y];

          let out = if out < 0 {
            0
          } else if out > 0xff {
            0xff
          } else {
            out as u8
          };

          dither_table[x][y][i as usize] = out;
        }
      }
    }

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
      image_transfer: Transfer::new(),
      cpu_transfer: Transfer::new(),
      vram: vec![0; VRAM_SIZE].into_boxed_slice(),
      picture: vec![0; 1024 * 512 * 3].into_boxed_slice(),
      texture_cache: [TextureCache::new(); 256],
      clut_tag: -1,
      clut_cache: [0; 256],
      current_clut: Coordinates2d::new(0, 0),
      current_texture_colors: TextureColors::FourBit,
      current_texture_x_base: 0,
      current_texture_y_base: 0,
      gpuread: 0,
      texture_window: 0,
      drawing_area_top_left: 0,
      drawing_area_bottom_right: 0,
      draw_offset: 0,
      debug_on: false,
      cpu_cycles: 0,
      dither_table,
      polyline: false,
      polyline_words_remaining: 0,
      polyline_prev_coord: Coordinates2d::new(0, 0),
      polyline_prev_color: RgbColor::new(0, 0, 0, false),
      polyline_shaded: false,
      polyline_semitransparent: false
    }
  }

  pub fn gpuread(&mut self) -> u32 {
    if self.cpu_transfer.is_active {
      let lower = self.transfer_to_cpu();
      let upper = self.transfer_to_cpu();

      return (lower as u32) | (upper as u32) << 16;
    }

    self.gpuread
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

  pub fn tick_counter(&mut self, cycles: i32, timers: &mut Timers) {
    self.cpu_cycles += cycles;

    // ticking for 250 cycles allows for less of a rounding error when multiplying by
    // the gpu to cpu cycles ratio. otherwise rendering appears to be too slow.
    if self.cpu_cycles > 250 {
      self.tick(self.cpu_cycles, timers);
      self.cpu_cycles = 0;
    }
  }

  fn tick(&mut self, cycles: i32, timers: &mut Timers) {
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
    let curr_x = self.image_transfer.x + self.image_transfer.read_x;
    let curr_y = self.image_transfer.y + self.image_transfer.read_y;

    self.image_transfer.read_x += 1;

    let vram_address = GPU::get_vram_address(curr_x & 0x3ff, curr_y & 0x1ff);

    if self.image_transfer.read_x == self.image_transfer.w {
      self.image_transfer.read_x = 0;

      self.image_transfer.read_y += 1;

      if self.image_transfer.read_y == self.image_transfer.h {
        self.image_transfer.is_active = false;
      }
    }

    self.vram[vram_address] = val as u8;
    self.vram[vram_address + 1] = (val >> 8) as u8;
  }

  fn transfer_to_cpu(&mut self) -> u16 {
    let x = self.cpu_transfer.x + self.cpu_transfer.read_x;
    let y = self.cpu_transfer.y + self.cpu_transfer.read_y;

    self.cpu_transfer.read_x += 1;

    if self.cpu_transfer.read_x == self.cpu_transfer.w {
      self.cpu_transfer.read_x = 0;

      self.cpu_transfer.read_y += 1;

      if self.cpu_transfer.read_y == self.cpu_transfer.h {
        self.cpu_transfer.is_active = false;
      }
    }

    let vram_address = GPU::get_vram_address(x, y);

    util::read_half(&self.vram, vram_address)
  }

  pub fn get_vram_address(x: u32, y: u32) -> usize {
    2 * (((x & 0x3ff) + 1024 * (y & 0x1ff))) as usize
  }

  pub fn get_vram_address_24(x: u32, y: u32) -> usize {
    (3 * (x & 0x3ff) + 2048 * (y & 0x1ff)) as usize
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

    if self.polyline {
      self.process_polyline(val);
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
      0x02 => self.gp0_fill_vram(),
      0x20..=0x3f => self.gp0_draw_polygon(),
      0x40..=0x5f => self.gp0_draw_line(),
      0x60..=0x7f => self.gp0_draw_rectangle(),
      0x80..=0x9f => self.gp0_vram_to_vram_transfer(),
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

  fn process_polyline(&mut self, word: u32) {
    if (word & 0xf000_f000) == 0x5000_5000 {
      self.polyline = false;
      return;
    }

    self.command_buffer[self.command_index] = word;
    self.command_index += 1;

    self.polyline_words_remaining -= 1;

    if self.polyline_words_remaining == 0 {

      let mut buffer_index = 0;

      let mut color2 = self.polyline_prev_color;

      if self.polyline_shaded {
        color2 = GPU::parse_color(self.command_buffer[buffer_index]);

        buffer_index += 1;
      }

      let end_position = self.parse_position(self.command_buffer[buffer_index]);

      self.rasterize_line(self.polyline_prev_coord, end_position, &mut [self.polyline_prev_color, color2], self.polyline_shaded, self.polyline_semitransparent);

      self.polyline_prev_coord = end_position;
      self.polyline_prev_color = color2;

      self.polyline_words_remaining = if self.polyline_shaded {
        2
      } else {
        1
      };

      self.command_index = 0;
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
      0x10..=0x1f => self.gp1_set_gpuread(val),
      _ => todo!("Invalid or unsupported GP1 command: {:02x}", op_code)
    }
  }

  fn gp1_set_gpuread(&mut self, val: u32) {
    self.gpuread = match val & 0x7 {
      0x02 => self.texture_window,
      0x03 => self.drawing_area_top_left,
      0x04 => self.drawing_area_bottom_right,
      0x05 => self.draw_offset,
      _ => self.gpuread
    }
  }

  fn gp1_clear_command_buffer(&mut self) {
    self.command_index = 0;
    self.words_remaining = 0;
  }

  fn gp1_acknowledge_interrupt(&mut self) {
    self.stat.irq_enabled = false;
  }

  fn gp0_vram_to_vram_transfer(&mut self) {
    let src = self.command_buffer[1];
    let dest = self.command_buffer[2];
    let dimensions = self.command_buffer[3];

    let src_x = src & 0x3ff;
    let src_y = (src >> 16) & 0x3ff;

    let dest_x = dest & 0x3ff;
    let dest_y = (dest >> 16) & 0x3ff;

    let mut w = dimensions & 0x3ff;
    let mut h = (dimensions >> 16) & 0x1ff;

    if w <= 0 {
      w = 0x400
    }
    if h <= 0 {
      h = 0x200;
    }

    for x in 0..w {
      for y in 0..h {
        let src_curr_x = src_x + x;
        let dest_curr_x = dest_x + x;

        let src_curr_y = src_y + y;
        let dest_curr_y = dest_y + y;

        let destination_address = GPU::get_vram_address(dest_curr_x, dest_curr_y);
        let source_address = GPU::get_vram_address(src_curr_x, src_curr_y);

        if self.stat.preserved_masked_pixels {
          let prev_color_upper_bits = self.vram[destination_address+1];

          if (prev_color_upper_bits >> 7) & 0b1 != 0 {
            continue;
          }
        }

        self.vram[destination_address] = self.vram[source_address];
        self.vram[destination_address + 1] = self.vram[source_address + 1];
      }
    }
  }

  fn gp0_fill_vram(&mut self) {
    let color = GPU::parse_color(self.command_buffer[0]);

    let destination = self.command_buffer[1];
    let dimensions = self.command_buffer[2];

    let pixel = GPU::color_to_u16(color);

    // clear out the lower 4 bits of x start per no&psx documents
    let x_start = destination & 0x3f0;
    let y_start = (destination >> 16) & 0x3ff;

    let w = ((dimensions & 0x3ff) + 0xf) & !0xf;
    let h = (dimensions >> 16) & 0x1ff;

    for y in 0..h {
      for x in 0..w {
        let vram_address = GPU::get_vram_address(x_start + x, y_start + y);
        self.vram[vram_address] = pixel as u8;
        self.vram[vram_address + 1] = (pixel >> 8) as u8;
      }
    }
  }

  fn gp0_image_transfer_to_cpu(&mut self) {
    let coordinates = self.command_buffer[1];
    let dimensions = self.command_buffer[2];

    let width = (dimensions & 0x3ff) as u32;
    let height = ((dimensions >> 16) & 0x1ff) as u32;

    let x = (coordinates & 0x3ff) as u32;
    let y = ((coordinates >> 16) & 0x1ff) as u32;

    // TODO: do something with this data

    self.cpu_transfer.h = height;
    self.cpu_transfer.w = width;

    self.cpu_transfer.x = x;
    self.cpu_transfer.y = y;

    self.cpu_transfer.read_x = 0;
    self.cpu_transfer.read_y = 0;

    self.cpu_transfer.is_active = true;
  }

  pub fn parse_color(val: u32) -> RgbColor {
    let r = val as u8;
    let g = (val >> 8) as u8;
    let b = (val >> 16) as u8;

    RgbColor::new(r, g, b, false)
  }

  pub fn parse_position(&self, val: u32) -> Coordinates2d {
    let mut x = (val & 0xffff) as i32;
    let mut y = (val >> 16) as i32;

    // sign extend
    x = (x << 21) >> 21;
    y = (y << 21) >> 21;

    let x_offset = self.drawing_x_offset as i32;
    let y_offset = self.drawing_y_offset as i32;

    Coordinates2d::new(x + x_offset, y + y_offset)
  }

  fn parse_texture_coords(command: u32) -> Coordinates2d {
    let x = (command & 0xff) as i32;
    let y = ((command >> 8) & 0xff) as i32;

    Coordinates2d::new(x, y)
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

    self.stat.semi_transparency = match (texture_data >> 5) & 0b11 {
      0 => SemiTransparency::Half,
      1 => SemiTransparency::Add,
      2 => SemiTransparency::Subtract,
      3 => SemiTransparency::AddQuarter,
      _ => unreachable!("can't happen")
    };
    self.stat.texture_x_base = (texture_data & 0xf) as u8;
    self.stat.texture_y_base1 = ((texture_data & 0x10)) as u8;
  }

  fn to_clut(command: u32) -> Coordinates2d {
    let clut = command >> 16;

    let x = (clut & 0x3f) * 16;
    let y = ((clut >> 6)) & 0x1ff;

    Coordinates2d::new(x as i32, y as i32)
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

    self.image_transfer.read_x = 0;
    self.image_transfer.read_y = 0;

    self.image_transfer.w = if w > 0 { w } else { 0x400 };
    self.image_transfer.h = if h > 0 { h } else { 0x200 };

    self.image_transfer.is_active = true;
  }

  fn gp0_mask_bit(&mut self) {
    let val = self.command_buffer[0];

    self.stat.set_mask_attributes(val);
  }

  fn gp0_draw_line(&mut self) {
    let word = self.command_buffer[0];

    let shaded = (word >> 28) & 0b1 == 1;
    self.polyline = (word >> 27) & 0b1 == 1;
    let semi_transparent = (word >> 25) & 0b1 == 1;

    self.polyline_words_remaining = if shaded {
      2
    } else {
      1
    };

    let color = GPU::parse_color(word);
    let mut color2 = color;

    let mut buffer_pos = 1;

    let start_position = self.parse_position(self.command_buffer[buffer_pos]);

    buffer_pos += 1;

    if shaded {
      color2 = GPU::parse_color(self.command_buffer[buffer_pos]);
      buffer_pos += 1;
    }

    let end_position = self.parse_position(self.command_buffer[buffer_pos]);

    self.rasterize_line(start_position, end_position, &mut [color, color2], shaded, semi_transparent);

    if self.polyline {
      self.polyline_prev_color = color2;
      self.polyline_prev_coord = end_position;
      self.polyline_shaded = shaded;
      self.polyline_semitransparent = semi_transparent;
    }

  }

  fn gp0_draw_rectangle(&mut self) {
    let word = self.command_buffer[0];

    let rectangle_size = (word >> 27) & 0x3;

    let textured = (word >> 26) & 0b1 == 1;
    let semi_transparent = (word >> 25) & 0b1 == 1;
    let blended = (word >> 24) & 0b1 == 0;

    let color = GPU::parse_color(self.command_buffer[0]);

    let mut tex_coordinates = Coordinates2d::new(0, 0);

    let coordinates = self.parse_position(self.command_buffer[1]);

    let mut command_pos = 2;

    let mut clut = Coordinates2d::new(0, 0);

    if textured {
      tex_coordinates = GPU::parse_texture_coords(self.command_buffer[command_pos]);

      clut = GPU::to_clut(self.command_buffer[command_pos]);

      if textured && (clut.x != self.current_clut.x||
        clut.y != self.current_clut.y ||
        self.stat.texture_x_base != self.current_texture_x_base ||
        self.stat.texture_y_base1 != self.current_texture_y_base ||
        self.stat.texture_colors != self.current_texture_colors) {
        self.gp0_invalidate_cache();
      }

      command_pos += 1;
    }

    let dimensions = match rectangle_size {
      0 => {
        let dimensions = self.command_buffer[command_pos];

        ((dimensions & 0x3ff), ((dimensions >> 16) & 0x1ff))
      },
      1 => (1,1),
      2 => (8,8),
      3 => (16,16),
      _ => unreachable!("can't happen")
    };

    self.rasterize_rectangle(color, coordinates, tex_coordinates, clut, dimensions, textured, blended, semi_transparent);
  }

  fn gp0_draw_polygon(&mut self) {
    let command = self.command_buffer[0] >> 24;

    let is_shaded = (command >> 4) & 0b1 == 1;
    let num_vertices = if (command >> 3) & 0b1 == 1 {
      4
    } else {
      3
    };

    let is_textured = (command >> 2) & 0b1 == 1;
    let semi_transparent = (command >> 1) & 0b1 == 1;
    let is_blended = command & 0b1 == 1;

    let color = GPU::parse_color(self.command_buffer[0]);

    let mut colors = [color, color, color, color];
    let mut positions: [Coordinates2d; 4] = [Coordinates2d::new(0, 0); 4];
    let mut tex_positions: [Coordinates2d; 4] = [Coordinates2d::new(0, 0); 4];

    let mut command_index = 0;

    let mut clut = Coordinates2d::new(0, 0);

    for i in 0..num_vertices {
      if i == 0 || is_shaded {
        colors[i] = GPU::parse_color(self.command_buffer[command_index]);
        command_index += 1;
      }

      positions[i] = self.parse_position(self.command_buffer[command_index]);

      command_index += 1;

      if is_textured {
        tex_positions[i] = GPU::parse_texture_coords(self.command_buffer[command_index]);

        if i == 0 {
          clut = GPU::to_clut(self.command_buffer[command_index]);
        } else if i == 1 {
          self.parse_texture_data(self.command_buffer[command_index]);
        }

        command_index += 1;
      }
    }

    if is_textured && (clut.x != self.current_clut.x ||
      clut.y != self.current_clut.y ||
      self.stat.texture_x_base != self.current_texture_x_base ||
      self.stat.texture_y_base1 != self.current_texture_y_base ||
      self.stat.texture_colors != self.current_texture_colors) {
      self.gp0_invalidate_cache();
    }

    let mut vertices = [
      Vertex::new(positions[0], colors[0], tex_positions[0]),
      Vertex::new(positions[1], colors[1], tex_positions[1]),
      Vertex::new(positions[2], colors[2], tex_positions[2]),
      Vertex::new(positions[3], colors[3], tex_positions[3]),
    ];

    let mut first_vertices = [
      vertices[0].clone(),
      vertices[1].clone(),
      vertices[2].clone()
    ];

    self.rasterize_triangle2(&mut first_vertices, clut, is_textured, is_shaded, is_blended, semi_transparent);

    if num_vertices == 4 {
      let mut second_vertices = [
        vertices[1].clone(),
        vertices[2].clone(),
        vertices[3].clone()
      ];
      self.rasterize_triangle2(&mut second_vertices, clut, is_textured, is_shaded, is_blended, semi_transparent);
    }
  }

  fn gp0_texture_window(&mut self) {
    let val = self.command_buffer[0];

    self.texture_window_x_mask = (val & 0x1f) as u8;
    self.texture_window_y_mask = ((val >> 5) & 0x1f) as u8;
    self.texture_window_x_offset = ((val >> 10) & 0x1f) as u8;
    self.texture_window_y_offset = ((val >> 15) & 0x1f) as u8;
    self.texture_window = val & 0xf_ffff;
  }

  fn gp0_draw_mode(&mut self) {
    let val = self.command_buffer[0];

    let previous_dither = self.stat.dither_enabled;

    self.stat.update_draw_mode(val);

    self.texture_rectangle_x_flip = ((val >> 12) & 0b1) == 1;
    self.texture_rectangle_y_flip = ((val >> 13) & 0b1) == 1;
  }

  fn gp0_draw_area_top_left(&mut self) {
    let val = self.command_buffer[0];

    self.drawing_area_top_left = val & 0x7_ffff;
    self.drawing_area_left = (val & 0x3ff) as u16;
    self.drawing_area_top = ((val >> 10) & 0x3ff) as u16;
  }

  fn gp0_draw_area_bottom_right(&mut self) {
    let val = self.command_buffer[0];

    self.drawing_area_bottom_right = val & 0x7_ffff;
    self.drawing_area_right = (val & 0x3ff) as u16;
    self.drawing_area_bottom = ((val >> 10) & 0x3ff) as u16;
  }

  fn gp0_drawing_offset(&mut self) {
    let val = self.command_buffer[0];

    let x = (val & 0x7ff) as u16;
    let y = ((val >> 11) & 0x7ff) as u16;

    self.draw_offset = val & 0x3f_ffff;

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