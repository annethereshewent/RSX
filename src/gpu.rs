use self::gpu_stat_register::GpuStatRegister;

pub mod gpu_stat_register;

/* per https://github.com/KieronJ/rpsx/blob/master/src/psx/gpu.rs */
const CMD_SIZE: [u32; 256] = [
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

enum GP0Mode {
  Command,
  ImageTransfer
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
  halfwords_remaining: u32,
  gp0_mode: GP0Mode
}

impl GPU {
  pub fn new() -> Self {
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
      display_horizontal_start: 0,
      display_horizontal_end: 0,
      display_line_start: 0,
      display_line_end: 0,
      texture_window_x_offset: 0,
      display_vram_x_start: 0,
      display_vram_y_start: 0,
      command_buffer: [0; 12],
      command_index: 0,
      words_remaining: 0,
      halfwords_remaining: 0,
      gp0_mode: GP0Mode::Command
    }
  }

  fn transfer_to_vram(&mut self, val: u16) {
    // TODO
  }

  pub fn gp0(&mut self, val: u32) {
    if matches!(self.gp0_mode, GP0Mode::ImageTransfer) {
      self.transfer_to_vram(val as u16);

      self.halfwords_remaining -= 1;

      if self.halfwords_remaining > 0 {
        self.transfer_to_vram((val >> 16) as u16);
        self.halfwords_remaining -= 1;
      }

      if self.halfwords_remaining == 0 {
        self.gp0_mode = GP0Mode::Command;
      }

      return;
    }

    self.command_buffer[self.command_index] = val;
    self.command_index += 1;

    if self.words_remaining == 0 {
      let op_code = val >> 24;
      self.words_remaining = CMD_SIZE[op_code as usize];
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
      0x01 => (), // clear cache, not implemented
      0x28 => self.gp0_monochrome_quadrilateral(),
      0x2c => self.textured_quad_with_blending(),
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
    self.gp0_mode = GP0Mode::Command;
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

  fn gp0_shaded_quadrilateral(&mut self) {
    // TODO
  }

  fn gp0_shaded_triangle(&mut self) {
    // TODO
  }

  fn textured_quad_with_blending(&mut self) {
    // TODO
  }

  fn gp0_image_transfer_to_vram(&mut self) {
    let _val = self.command_buffer[0];
    // TODO: add coordinates from command buffer index 1
    let dimensions = self.command_buffer[2];

    let w = dimensions & 0xffff;
    let h = dimensions >> 16;

    let image_size = w * h;

    self.halfwords_remaining = image_size;

    // TODO: actually transfer image data to vram
    self.gp0_mode = GP0Mode::ImageTransfer;
  }

  fn gp0_mask_bit(&mut self) {
    let val = self.command_buffer[0];

    self.stat.set_mask_attributes(val);
  }

  fn gp0_monochrome_quadrilateral(&mut self) {
    // TODO
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

    // TODO: invalidate GPU cache and clear command FIFO when implemented

  }
}