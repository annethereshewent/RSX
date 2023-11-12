use std::cmp;

use super::{GPU, gpu_stat_register::{ColorDepth, TextureColors}};

impl GPU {
  fn get_2d_area(pos1: (i32, i32), pos2: (i32, i32), pos3: (i32, i32)) -> i32 {
    (pos2.0 - pos1.0) * (pos3.1 - pos1.1) - (pos2.1 - pos1.1) * (pos3.0 - pos1.0)
  }

  pub fn update_picture(&mut self) {
    let mut x_start = self.display_vram_x_start as u32;
    let mut y_start = self.display_vram_y_start as u32;

    x_start += ((self.display_horizontal_start as u32) - 608) / (self.get_dotclock() as u32);
    y_start += ((self.display_line_start as u32) - 16) * 2;

    let (w, h) = self.get_dimensions();

    let mut i = 0;

    for y in y_start..y_start + h {
      for x in x_start..x_start + w {
        match self.stat.display_color_depth {
          ColorDepth::FifteenBit => {
            let vram_address = self.get_vram_address(x as u32, y as u32);

            let color = (self.vram[vram_address] as u16) | (self.vram[vram_address + 1] as u16) << 8;

            let (r, g, b) = self.translate_15bit_to_24(color);

            self.picture[i] = r;
            self.picture[i + 1] = g;
            self.picture[i + 2] = b;

          }
          ColorDepth::TwentyFourBit => {
            let vram_address = self.get_vram_address_24(x as u32, y as u32);

            self.picture[i] = self.vram[vram_address];
            self.picture[i + 1] = self.vram[vram_address + 1];
            self.picture[i + 2] = self.vram[vram_address + 2];
          }
        }
        i += 3;
      }
    }
  }

  fn translate_15bit_to_24(&self, val: u16) -> (u8, u8, u8) {
    let mut r = (val & 0x1f) as u8;
    let mut g = ((val >> 5) & 0x1f) as u8;
    let mut b = ((val >> 10) & 0x1f) as u8;

    r = (r << 3) | (r >> 2);
    g = (g << 3) | (g >> 2);
    b = (b << 3) | (b >> 2);

    (r, g, b)
  }

  pub fn get_dimensions(&self) -> (u32, u32) {
    let dotclock = self.get_dotclock() as u32;
    let mut w = if self.display_horizontal_start <= self.display_horizontal_end {
      self.display_horizontal_end - self.display_horizontal_start
    } else {
      50
    } as u32;

    w = ((w / dotclock) + 2) & !0b11;
    let mut h = (self.display_line_end - self.display_line_start) as u32;

    if self.stat.vertical_interlace {
      h *= 2;
    }

    (w, h)
  }

  pub fn render_pixel(&mut self, position: (i32, i32), color: (u8, u8, u8)) {
    let vram_address = self.get_vram_address(position.0 as u32, position.1 as u32);

    let value = GPU::color_to_u16(color);

    self.vram[vram_address] = value as u8;
    self.vram[vram_address + 1] = (value >> 8) as u8;
  }

  pub fn rasterize_triangle(&mut self, c: &mut [(u8, u8, u8)], p: &mut [(i32, i32)], t: Option<&mut [(i32,i32)]>, clut: Option<(i32, i32)>, is_shaded: bool, is_blended: bool) {
    let mut area = GPU::get_2d_area(p[0], p[1], p[2]);

    let mut no_textures = [(0,0), (0,0), (0,0)];

    let t = if t.is_some() {
      t.unwrap()
    } else {
      &mut no_textures
    };

    if area == 0 {
      return;
    }

    if area < 0 {
      p.swap(1, 2);
      c.swap(1, 2);
      t.swap(1, 2);

      area = -area;
    }

    let mut min_x = cmp::min(p[0].0, cmp::min(p[1].0, p[2].0));
    let mut min_y = cmp::min(p[0].1, cmp::min(p[1].1, p[2].1));

    let mut max_x = cmp::max(p[0].0, cmp::max(p[1].0, p[2].0));
    let mut max_y = cmp::max(p[0].1, cmp::max(p[1].1, p[2].1));

    let diff_x = max_x - min_x;
    let diff_y = max_y - min_y;

    if min_x >= 1024 ||
      max_x >= 1024 ||
      min_y >= 512 ||
      min_x >= 512 ||
      min_x < 0 ||
      min_y < 0 ||
      max_x < 0 ||
      min_y < 0 ||
      diff_x >= 1024 ||
      diff_y >= 512 {
        return;
    }

    min_x = cmp::max(min_x, self.drawing_area_left as i32);
    min_y = cmp::max(min_y, self.drawing_area_top as i32);

    max_x = cmp::min(max_x, self.drawing_area_right as i32);
    max_y = cmp::min(max_y, self.drawing_area_bottom as i32);

    let a01 = (p[0].1 - p[1].1) as i32;
    let b01 = (p[1].0 - p[0].0) as i32;

    let a12 = (p[1].1 - p[2].1) as i32;
    let b12 = (p[2].0 - p[1].0) as i32;

    let a20 = (p[2].1 - p[0].1) as i32;
    let b20 = (p[0].0 - p[2].0) as i32;

    let mut curr_p = (min_x, min_y);

    let mut w0_row = GPU::get_2d_area(p[1], p[2], curr_p) as i32;
    let mut w1_row = GPU::get_2d_area(p[2], p[0], curr_p) as i32;
    let mut w2_row = GPU::get_2d_area(p[0], p[1], curr_p) as i32;

    let w0_bias = -(GPU::is_top_left(b12, a12) as i32);
    let w1_bias = -(GPU::is_top_left(b20, a20) as i32);
    let w2_bias = -(GPU::is_top_left(b01, a01) as i32);

    let blend_color = c[0];
    let mut output = blend_color;

    while curr_p.1 < max_y {
      curr_p.0 = min_x;

      let mut w0 = w0_row;
      let mut w1 = w1_row;
      let mut w2 = w2_row;
      while curr_p.0 < max_x {
        if ((w0 + w0_bias) | (w1 + w1_bias) | (w2 + w2_bias)) >= 0 {
          let vec_3d = (w0, w1, w2);

          if is_shaded {
            output = GPU::interpolate_color(area as i32, vec_3d, c[0], c[1], c[2]);
          }

          if let Some(clut) = clut {
            let mut uv = GPU::interpolate_texture_coordinates(area, vec_3d, t[0], t[1], t[2]);
            uv = self.mask_texture_coordinates(uv);

            if let Some(mut texture) = self.get_texture(uv, clut) {
              if is_blended {
                texture.0 = (((texture.0 as u32) * (blend_color.0 as u32)) >> 7) as u8;
                texture.1 = (((texture.1 as u32) * (blend_color.1 as u32)) >> 7) as u8;
                texture.2 = (((texture.2 as u32) * (blend_color.2 as u32)) >> 7) as u8;
              }

              output = texture;
            } else {
              w0 += a12;
              w1 += a20;
              w2 += a01;

              curr_p.0 += 1;
              continue;
            }
          }

          self.render_pixel(curr_p, output);
        }
        w0 += a12;
        w1 += a20;
        w2 += a01;

        curr_p.0 += 1;
      }
      w0_row += b12;
      w1_row += b20;
      w2_row += b01;

      curr_p.1 += 1;
    }
  }

  fn color_to_u16(color: (u8, u8, u8)) -> u16 {
    let mut pixel = 0;

    pixel |= ((color.0 as u16) & 0xf8) >> 3;
    pixel |= ((color.1 as u16) & 0xf8) << 2;
    pixel |= ((color.2 as u16) & 0xf8) << 7;

    pixel
  }

  fn is_top_left(x: i32, y: i32) -> bool {
    (y < 0) || ((x < 0) && (y == 0))
  }

  fn interpolate_color(area: i32, vec_3d: (i32,i32,i32), color0: (u8,u8,u8), color1: (u8,u8,u8), color2: (u8,u8,u8)) -> (u8, u8, u8) {
    let color0_r = color0.0 as i32;
    let color1_r = color1.0 as i32;
    let color2_r = color2.0 as i32;

    let color0_g = color0.1 as i32;
    let color1_g = color1.1 as i32;
    let color2_g = color2.1 as i32;

    let color0_b = color0.2 as i32;
    let color1_b = color1.2 as i32;
    let color2_b = color2.2 as i32;


    let r = ((vec_3d.0 * color0_r + vec_3d.1 * color1_r + vec_3d.2 * color2_r) / area) as u8;
    let g = ((vec_3d.0 * color0_g + vec_3d.1 * color1_g + vec_3d.2 * color2_g) / area) as u8;
    let b = ((vec_3d.0 * color0_b + vec_3d.1 * color1_b + vec_3d.2 * color2_b) / area) as u8;

    (r,g,b)
  }

  fn interpolate_texture_coordinates(area: i32, w: (i32, i32, i32),
    t0: (i32, i32),
    t1: (i32, i32),
    t2: (i32, i32)) -> (i32, i32) {
    let u = (w.0 * t0.0 + w.1 * t1.0 + w.2 * t2.0) / area;
    let v = (w.0 * t0.1 + w.1 * t1.1 + w.2 * t2.1) / area;

    (u, v)
  }

  fn mask_texture_coordinates(&self, mut uv: (i32, i32)) -> (i32, i32) {
    let mask_x = self.texture_window_x_mask as i32;
    let mask_y = self.texture_window_y_mask as i32;

    let offset_x = self.texture_window_x_offset as i32;
    let offset_y = self.texture_window_y_offset as i32;

    uv.0 = (uv.0 & !mask_x) | (offset_x & mask_x);
    uv.1 = (uv.1 & !mask_y) | (offset_y & mask_y);

    uv
  }

  fn get_texture(&mut self, uv: (i32, i32), clut: (i32, i32)) -> Option<(u8,u8,u8)> {
    match self.stat.texture_colors {
      TextureColors::FourBit => self.read_4bit_clut(uv, clut),
      TextureColors::EightBit => self.read_8bit_clut(uv, clut),
      TextureColors::FifteenBit => self.read_texture(uv)
    }
  }

  fn read_4bit_clut(&mut self, uv: (i32, i32), clut: (i32, i32)) -> Option<(u8,u8,u8)> {
    let tex_x_base = (self.stat.texture_x_base as i32) * 64;
    let tex_y_base = (self.stat.texture_y_base1 as i32) * 16;

    let offset_x = (2 * tex_x_base + ((uv.0 / 2) & 0xff)) as u32;
    let offset_y = (tex_y_base + (uv.1 & 0xff)) as u32;

    let texture_address = (offset_x + 2048 * offset_y) as usize;

    let block = (((uv.1 >> 6) << 2) + (uv.0 >> 6)) as isize;
    let entry = (((uv.1 & 0x3f) << 2) + ((uv.0 & 0x3f) >> 4)) as usize;

    let index = ((uv.0 >> 1) & 0x7) as usize;

    let cache_entry = &mut self.texture_cache[entry];

    if cache_entry.tag != block {
      for i in 0..8 {
        cache_entry.data[i] = self.vram[(texture_address & !0b111) + i];
      }

      cache_entry.tag = block;
    }

    let mut clut_entry = cache_entry.data[index] as usize;

    if (uv.0 & 0b1) != 0 {
      clut_entry >>= 4;
    } else {
      clut_entry &= 0xf;
    }

    let clut_address = (2 * clut.0 + 2048 * clut.1) as isize;

    if self.clut_tag != clut_address {
      for i in 0..16 {
        let address = (clut_address as usize) + 2 * i;
        self.clut_cache[i] = (self.vram[address] as u16) | (self.vram[address + 1] as u16) << 8;
      }

      self.clut_tag = clut_address;
    }

    let texture = self.clut_cache[clut_entry];

    if texture != 0 {
      Some(self.translate_15bit_to_24(texture))
    } else {
      None
    }
  }

  fn read_8bit_clut(&self, _uv: (i32, i32), _clut: (i32, i32)) -> Option<(u8,u8,u8)> {
    Some((0,0,0))
  }

  fn read_texture(&self, _uv: (i32, i32)) -> Option<(u8,u8,u8)> {
    Some((0,0,0))
  }
}