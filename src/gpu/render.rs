use std::{cmp, mem};

use crate::util;

use super::{GPU, gpu_stat_register::{ColorDepth, TextureColors, SemiTransparency}, RgbColor, Coordinates2d, Coordinates3d, Vertex};

impl GPU {
  fn cross_product(a: Coordinates2d, b: Coordinates2d, c: Coordinates2d) -> i32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
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
            let vram_address = GPU::get_vram_address(x as u32, y as u32);

            let color = util::read_half(&self.vram, vram_address);

            let color = GPU::translate_15bit_to_24(color);

            self.picture[i] = color.r;
            self.picture[i + 1] = color.g;
            self.picture[i + 2] = color.b;

          }
          ColorDepth::TwentyFourBit => {
            let vram_address = GPU::get_vram_address_24(x as u32, y as u32);

            self.picture[i] = self.vram[vram_address];
            self.picture[i + 1] = self.vram[vram_address + 1];
            self.picture[i + 2] = self.vram[vram_address + 2];
          }
        }
        i += 3;
      }
    }
  }

  fn translate_15bit_to_24(val: u16) -> RgbColor {
    let mut r = (val & 0x1f) as u8;
    let mut g = ((val >> 5) & 0x1f) as u8;
    let mut b = ((val >> 10) & 0x1f) as u8;
    let a = (val >> 15) & 0b1 == 1;

    r = (r << 3) | (r >> 2);
    g = (g << 3) | (g >> 2);
    b = (b << 3) | (b >> 2);

    RgbColor::new(r, g, b, a)
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

  pub fn render_pixel(&mut self, position: Coordinates2d, color: RgbColor, textured: bool, semi_transparent: bool) {
    let vram_address = GPU::get_vram_address(position.x as u32, position.y as u32);

    let mut color = color;

    if self.stat.preserved_masked_pixels && color.a {
      return;
    }

    if (!textured || color.a) && semi_transparent {
      let val = util::read_half(&self.vram, vram_address);
      let prev_color = GPU::translate_15bit_to_24(val);

      let (r,g, b) = match self.stat.semi_transparency {
        SemiTransparency::Half => {
          (
            GPU::add_half_semitransparency(prev_color.r, color.r),
            GPU::add_half_semitransparency(prev_color.g, color.g),
            GPU::add_half_semitransparency(prev_color.b, color.b)
          )
        }
        SemiTransparency::Add => {
          (
            GPU::add_semitransparency(prev_color.r, color.r),
            GPU::add_semitransparency(prev_color.g, color.g),
            GPU::add_semitransparency(prev_color.b, color.b)
          )
        }
        SemiTransparency::Subtract => {
          (
            GPU::subtract_semitransparency(prev_color.r, color.r),
            GPU::subtract_semitransparency(prev_color.g, color.g),
            GPU::subtract_semitransparency(prev_color.b, color.b)
          )
        }
        SemiTransparency::AddQuarter => {
          (
            GPU::add_quarter_semitransparency(prev_color.r, color.r),
            GPU::add_quarter_semitransparency(prev_color.g, color.g),
            GPU::add_quarter_semitransparency(prev_color.b, color.b),
          )
        }
      };

      color.r = r;
      color.g = g;
      color.b = b;
    }

    if self.stat.force_mask_bit {
      color.a = true;
    }

    let value = GPU::color_to_u16(color);

    self.vram[vram_address] = value as u8;
    self.vram[vram_address + 1] = (value >> 8) as u8;
  }


  fn add_half_semitransparency(x: u8, y: u8) -> u8 {
    cmp::min(255,(x as u32 + y as u32) / 2) as u8
  }

  fn add_semitransparency(x: u8, y: u8) -> u8 {
    cmp::min(255, x as u32 + y as u32) as u8
  }

  fn subtract_semitransparency(x: u8, y: u8) -> u8 {
    let result = x as i32 - y as i32;

    if result < 0 {
      return 0;
    }

    result as u8
  }

  fn add_quarter_semitransparency(x: u8, y: u8) -> u8 {
    cmp::min(255, (x as u32 + y as u32) / 4) as u8
  }

  pub fn rasterize_line(&mut self, mut start_position: Coordinates2d, mut end_position: Coordinates2d, colors: &mut [RgbColor], shaded: bool, semi_transparent: bool) {
    if start_position.x > end_position.x {
      mem::swap(&mut start_position, &mut end_position);
    }

    let start_x = start_position.x;
    let start_y = start_position.y;

    let end_x = end_position.x;
    let end_y = end_position.y;


    let diff_x = end_x - start_x;
    let diff_y = end_y - start_y;

    // no need to use .abs() on diff_x, as end_x is guaranteed to be after start_x due to mem swap above
    if diff_x > 1024 || diff_y.abs() > 512 || (diff_x == 0 && diff_y == 0) {
      return;
    }

    let mut color_r_fp = (colors[0].r as i32) << 12;
    let mut color_g_fp = (colors[0].g as i32) << 12;
    let mut color_b_fp = (colors[0].b as i32) << 12;

    if diff_x != 0 {
      // so basically, to draw the line, get the slope of the line and follow the slope and render the line that way.
      // convert to fixed point to be less resource intensive than using floating point
      let slope = ((diff_y as i64) << 12) / diff_x as i64;

      // for the colors we can do something similar and create a "slope" based on the x coordinate and the difference between the rgb color values
      // use fixed point to make conversion to u8 easier
      let r_slope = (((colors[1].r - colors[0].r) as i32) << 12) / diff_x;
      let g_slope = (((colors[1].g - colors[0].g) as i32) << 12) / diff_x;
      let b_slope = (((colors[1].b - colors[0].b) as i32) << 12) / diff_x;

      let mut color = colors[0];

      for x in start_x..=end_x {
        color.r = (color_r_fp >> 12) as u8;
        color.g = (color_g_fp >> 12) as u8;
        color.b = (color_b_fp >> 12) as u8;

        let y = ((x as i64 * slope) >> 12) as i32 + start_y;

        if shaded {
          color_r_fp += r_slope;
          color_g_fp += g_slope;
          color_b_fp += b_slope;
        }

        if self.stat.dither_enabled {
          self.dither(Coordinates2d::new(x, y), &mut color);
        }

        self.render_pixel(Coordinates2d::new(x, y), color, false, semi_transparent);
      }
    } else {
      // line is vertical, just render from start y to end y

      // create a "slope" based on the change in y instead of change in x
      let diff_y = diff_y.abs();

      let r_slope = (((colors[1].r - colors[0].r) as i32) << 12) / diff_y;
      let g_slope = (((colors[1].g - colors[0].g) as i32) << 12) / diff_y;
      let b_slope = (((colors[1].b - colors[0].b) as i32) << 12) / diff_y;

      let mut color = colors[0];

      let going_up = start_y > end_y;

      for y_offset in 0..=diff_y {
        let y = if going_up {
          start_y - y_offset
        } else {
          start_y + y_offset
        };

        color.r = (color_r_fp >> 12) as u8;
        color.g = (color_g_fp >> 12) as u8;
        color.b = (color_b_fp >> 12) as u8;

        if shaded {
          color_r_fp += r_slope;
          color_g_fp += g_slope;
          color_b_fp += b_slope;
        }

        let position = Coordinates2d::new(start_x, y);

        if self.stat.dither_enabled {
          self.dither(position, &mut color);
        }

        self.render_pixel(position, color, false, semi_transparent);
      }
    }
  }

  pub fn rasterize_rectangle(&mut self, color: RgbColor, coordinates: Coordinates2d, tex_coordinates: Coordinates2d, clut: Coordinates2d, dimensions: (u32, u32), textured: bool, blended: bool, semi_transparent: bool) {
    for x in 0..dimensions.0 {
      for y in 0..dimensions.1 {
        let curr_x = coordinates.x + x as i32;
        let curr_y = coordinates.y + y as i32;

        if curr_x < self.drawing_area_left as i32 || curr_y < self.drawing_area_top as i32 || curr_x >= self.drawing_area_right as i32 || curr_y >= self.drawing_area_bottom as i32 {
          continue;
        }

        let mut output = color;

        if textured {
          let mut uv = Coordinates2d::new(tex_coordinates.x + (x & 0xff) as i32, tex_coordinates.y + (y & 0xff) as i32);
          uv = self.mask_texture_coordinates(uv);

          if let Some(mut texture) = self.get_texture(uv, clut) {
            if blended {
              GPU::blend_colors(&mut texture, &color);
            }
            output = texture;
          } else {
            continue;
          }
        }
        self.render_pixel(Coordinates2d::new(curr_x, curr_y), output, textured, semi_transparent);
      }
    }
  }

  pub fn rasterize_triangle(&mut self, v: &mut [Vertex], clut: Coordinates2d, is_textured: bool, is_shaded: bool, is_blended: bool, semi_transparent: bool) {
    // sort the vertices by y position so the first vertex is always the top most one
    v.sort_by(|a, b| a.p.y.cmp(&b.p.y));

    let p: &mut Vec<Coordinates2d> = &mut v.iter().map(|vertex| vertex.p).collect();
    let c: &Vec<RgbColor> = &v.iter().map(|vertex| vertex.c).collect();
    let t: &Vec<Coordinates2d> = &v.iter().map(|vertex| vertex.uv).collect();

    let cross_product = GPU::cross_product(p[0], p[1], p[2]);

    if cross_product == 0 {
      return;
    }

    let mut min_x = cmp::min(p[0].x, cmp::min(p[1].x, p[2].x));
    let mut min_y = cmp::min(p[0].y, cmp::min(p[1].y, p[2].y));

    let mut max_x = cmp::max(p[0].x, cmp::max(p[1].x, p[2].x));
    let mut max_y = cmp::max(p[0].y, cmp::max(p[1].y, p[2].y));

    let diff_x = max_x - min_y;
    let diff_y = max_y - min_y;

    if min_x < 0 || min_y < 0 || max_x >= 1024 || max_y >= 512 || (diff_x == 0 && diff_y == 0) {
      return;
    }

    let drawing_area_left = self.drawing_area_left as i32;
    let drawing_area_top = self.drawing_area_top as i32;
    let drawing_area_right = self.drawing_area_right as i32;
    let drawing_area_bottom = self.drawing_area_bottom as i32;

    if min_x < drawing_area_left {
      min_x = drawing_area_left;
    }

    if min_y < drawing_area_top {
      min_y = drawing_area_top;
    }

    if max_x > drawing_area_right {
      max_x = drawing_area_right;
    }

    if max_y > drawing_area_bottom {
      max_y = drawing_area_bottom;
    }

    // using fixed point arithmetic for performance boost, also hoping to fix some bugs with textures
    let mut drdx = 0;
    let mut drdy = 0;

    let mut dgdx = 0;
    let mut dgdy = 0;

    let mut dbdx = 0;
    let mut dbdy = 0;

    if is_shaded {
      (drdx, drdy, dgdx, dgdy, dbdx, dbdy) = GPU::get_color_deltas(&v, cross_product);
    }

    let mut dudx = 0;
    let mut dudy = 0;

    let mut dvdx = 0;
    let mut dvdy = 0;

    if is_textured {
      (dudx, dudy, dvdx, dvdy) = GPU::get_texture_deltas(p, t, cross_product);
    }

    // set the base u/v and rgb colors relative to 0,0 so it's easier to convert. use the first vertex as the reference
    let mut r_base = (c[0].r as i64) << 12;
    let mut g_base = (c[0].g as i64) << 12;
    let mut b_base = (c[0].b as i64) << 12;

    let uv_base = t[0];

    let mut u_base_fp = (uv_base.x as i64) << 12;
    let mut v_base_fp = (uv_base.y as i64) << 12;

    // add .5 for rounding
    u_base_fp |= 1 << 11;
    v_base_fp |= 1 << 11;

    r_base -= drdx * p[0].x as i64;
    r_base -= drdy * p[0].y as i64;

    g_base -= dgdx * p[0].x as i64;
    g_base -= dgdy * p[0].y as i64;

    b_base -= dbdx * p[0].x as i64;
    b_base -= dbdy * p[0].y as i64;

    u_base_fp -= dudx * p[0].x as i64 + dudy * p[0].y as i64;
    v_base_fp -= dvdx * p[0].x as i64 + dvdy * p[0].y as i64;

    // using fixed point for better performance (and to hopefully fix some bugs)
    let p01_slope = if p[0].y != p[1].y {
      let dx_fp = ((p[1].x - p[0].x) as i64) << 12;
      Some(dx_fp / (p[1].y - p[0].y) as i64)
    } else {
      None
    };

    let p02_slope = if p[0].y != p[2].y {
      let dx_fp = ((p[2].x - p[0].x) << 12) as i64;
      Some(dx_fp / (p[2].y - p[0].y) as i64)
    } else {
      None
    };

    let p12_slope = if p[1].y != p[2].y {
      let dx_fp = ((p[2].x - p[1].x) << 12) as i64;
      Some(dx_fp / (p[2].y - p[1].y) as i64)
    } else {
      None
    };

    let p02_is_left = cross_product > 0;

    let mut curr_p = Coordinates2d::new(min_x, min_y);

    let mut output = c[0];

    while curr_p.y < max_y {
      curr_p.x = min_x;
      while curr_p.x < max_x {
        let (boundary1, boundary2) = self.get_triangle_boundaries(p, p01_slope, p12_slope, p02_slope, curr_p);

        let (curr_min_x, curr_max_x) = if p02_is_left {
          (boundary1, boundary2)
        } else {
          (boundary2, boundary1)
        };

        if curr_p.x >= curr_min_x && curr_p.x < curr_max_x {
          // render the pixel
          if is_shaded {
            GPU::interpolate_color(&mut output, curr_p, r_base, g_base, b_base, drdx, drdy, dgdx, dgdy, dbdx, dbdy);

            if self.stat.dither_enabled {
              self.dither(curr_p, &mut output);
            }
          }

          if is_textured {
            let mut uv = GPU::interpolate_texture_coordinates(curr_p, u_base_fp, v_base_fp, dudx, dudy, dvdx, dvdy);

            uv = self.mask_texture_coordinates(uv);

            if let Some(mut texture) = self.get_texture(uv, clut) {
              if is_blended {
                GPU::blend_colors(&mut texture, &output);

                if self.stat.dither_enabled {
                  self.dither(curr_p, &mut texture);
                }
              }

              output = texture;
            } else {
              curr_p.x += 1;
              continue;
            }
          }

          self.render_pixel(curr_p, output, is_textured, semi_transparent);
        }
        curr_p.x += 1;
      }
      curr_p.y += 1;
    }
  }

  fn interpolate_color(output: &mut RgbColor, curr_p: Coordinates2d, r_base: i64, g_base: i64, b_base: i64, drdx: i64, drdy: i64, dgdx: i64, dgdy: i64, dbdx: i64, dbdy: i64) {
    output.r = ((drdx * curr_p.x as i64 + drdy * curr_p.y as i64 + r_base) >> 12) as u8;
    output.g = ((dgdx * curr_p.x as i64 + dgdy * curr_p.y as i64 + g_base) >> 12) as u8;
    output.b = ((dbdx * curr_p.x as i64 + dbdy * curr_p.y as i64 + b_base) >> 12) as u8;
  }

  fn interpolate_texture_coordinates(curr_p: Coordinates2d, u_base_fp: i64, v_base_fp: i64, dudx: i64, dudy: i64, dvdx: i64, dvdy: i64) -> Coordinates2d {
    // converting to u8 is a really shitty hack that will ensure that texture coordinates aren't out of bounds.
    // this will definitely cause rendering issues, but I'm not sure of a way around it atm
    // TODO: find a way around this hack
    let u = ((curr_p.x as i64 * dudx + curr_p.y as i64 * dudy + u_base_fp) >> 12) as u8 as i32;

    let v = ((curr_p.x as i64 * dvdx + curr_p.y as i64 * dvdy + v_base_fp) >> 12) as u8 as i32;

    Coordinates2d::new(u, v)
  }

  fn get_color_deltas(v: &[Vertex], cross_product: i32) -> (i64, i64, i64, i64, i64, i64) {
    let drdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].c.r as i32, v[0].p.y),
      Coordinates2d::new(v[1].c.r as i32, v[1].p.y),
      Coordinates2d::new(v[2].c.r as i32, v[2].p.y)
    );

    let drdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].c.r as i32),
      Coordinates2d::new(v[1].p.x, v[1].c.r as i32),
      Coordinates2d::new(v[2].p.x, v[2].c.r as i32)
    );

    let dgdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].c.g as i32, v[0].p.y),
      Coordinates2d::new(v[1].c.g as i32, v[1].p.y),
      Coordinates2d::new(v[2].c.g as i32, v[2].p.y)
    );

    let dgdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].c.g as i32),
      Coordinates2d::new(v[1].p.x, v[1].c.g as i32),
      Coordinates2d::new(v[2].p.x, v[2].c.g as i32)
    );

    let dbdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].c.b as i32, v[0].p.y),
      Coordinates2d::new(v[1].c.b as i32, v[1].p.y),
      Coordinates2d::new(v[2].c.b as i32, v[2].p.y)
    );

    let dbdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].c.b as i32),
      Coordinates2d::new(v[1].p.x, v[1].c.b as i32),
      Coordinates2d::new(v[2].p.x, v[2].c.b as i32)
    );


    let drdx = (drdx_cp << 12) as i64 / cross_product as i64;
    let drdy = (drdy_cp << 12) as i64 / cross_product as i64;

    let dgdx = (dgdx_cp << 12) as i64 / cross_product as i64;
    let dgdy = (dgdy_cp << 12) as i64 / cross_product as i64;

    let dbdx = (dbdx_cp << 12) as i64 / cross_product as i64;
    let dbdy = (dbdy_cp << 12) as i64 / cross_product as i64;

    (drdx, drdy, dgdx, dgdy, dbdx, dbdy)

  }

  fn get_texture_deltas(p: &mut [Coordinates2d], t: &[Coordinates2d], cross_product: i32) -> (i64, i64, i64, i64) {
    let dudx_cp = GPU::cross_product(
      Coordinates2d::new(t[0].x, p[0].y),
      Coordinates2d::new(t[1].x, p[1].y),
      Coordinates2d::new(t[2].x, p[2].y)
    );

    let dudy_cp = GPU::cross_product(
      Coordinates2d::new(p[0].x, t[0].x),
      Coordinates2d::new(p[1].x, t[1].x),
      Coordinates2d::new(p[2].x, t[2].x)
    );

    let dvdx_cp = GPU::cross_product(
      Coordinates2d::new(t[0].y, p[0].y),
      Coordinates2d::new(t[1].y, p[1].y),
      Coordinates2d::new(t[2].y, p[2].y)
    );

    let dvdy_cp = GPU::cross_product(
      Coordinates2d::new(p[0].x, t[0].y),
      Coordinates2d::new(p[1].x, t[1].y),
      Coordinates2d::new(p[2].x, t[2].y)
    );

    let dudx = (dudx_cp << 12) as i64 / cross_product as i64;
    let dudy = (dudy_cp << 12) as i64 / cross_product as i64;

    let dvdx = (dvdx_cp << 12) as i64 / cross_product as i64;
    let dvdy = (dvdy_cp << 12) as i64 / cross_product as i64;

    (dudx, dudy, dvdx, dvdy)
  }

  /**
   * Gets the triangle left and right boundaries depending on the slopes.
   * Since p02 slope can either be on the left or right side of the triangle,
   * to get the min/max for either side, just switch the boundaries. (ie: if
   * p02 is on the left, then max is boundary2, min is boundary1, other way otherwise).
   */
  fn get_triangle_boundaries(&self, p: &[Coordinates2d], p01_slope: Option<i64>, p12_slope: Option<i64>, p02_slope: Option<i64>, curr_p: Coordinates2d) -> (i32, i32) {
    // consider the following cases:

    // p01 is horizontal
    // p12 is horizontal
    // neither are horizontal
    // p02 can't be horizontal because vertices are always sorted by y coordinate, hence line p02 is always either vertical or diagonal.
    let boundary2 = if p01_slope.is_none() {
      let rel_y = (curr_p.y - p[1].y) as i64;
      // use p12 slope
      let slope = p12_slope.unwrap();

      ((slope * rel_y) >> 12) as i32 + p[1].x
    } else if p12_slope.is_none() {
      let rel_y = (curr_p.y - p[0].y) as i64;

      let slope = p01_slope.unwrap();

      ((slope * rel_y) >> 12) as i32 + p[0].x
    } else {
      // determine what slope to use based on y coordinate
      if curr_p.y <= p[1].y {
        // use p01 slope
        let rel_y = (curr_p.y - p[0].y) as i64;
        let slope = p01_slope.unwrap();

        ((slope * rel_y) >> 12) as i32 + p[0].x
      } else {
        // use p12 slope
        let rel_y = (curr_p.y - p[1].y) as i64;

        let slope = p12_slope.unwrap();

        ((slope * rel_y) >> 12) as i32 + p[1].x
      }
    };

    let rel_y = (curr_p.y - p[0].y) as i64;
    let slope = p02_slope.unwrap();

    let boundary1 = ((slope * rel_y) >> 12) as i32 + p[0].x;

    (boundary1, boundary2)
  }

  fn blend_colors(texture: &mut RgbColor, color: &RgbColor) {
    texture.r = cmp::min(255,((texture.r as u32) * (color.r as u32)) >> 7) as u8;
    texture.g = cmp::min(255, ((texture.g as u32) * (color.g as u32)) >> 7) as u8;
    texture.b = cmp::min(255, ((texture.b as u32) * (color.b as u32)) >> 7) as u8;
  }

  pub fn color_to_u16(color: RgbColor) -> u16 {
    let mut pixel = 0;

    pixel |= ((color.r as u16) & 0xf8) >> 3;
    pixel |= ((color.g as u16) & 0xf8) << 2;
    pixel |= ((color.b as u16) & 0xf8) << 7;

    pixel
  }

  fn is_top_left(x: i32, y: i32) -> bool {
    (y < 0) || ((x < 0) && (y == 0))
  }

  fn mask_texture_coordinates(&self, mut uv: Coordinates2d) -> Coordinates2d {
    let mask_x = self.texture_window_x_mask as i32;
    let mask_y = self.texture_window_y_mask as i32;

    let offset_x = self.texture_window_x_offset as i32;
    let offset_y = self.texture_window_y_offset as i32;

    uv.x = (uv.x & !mask_x) | (offset_x & mask_x);
    uv.y = (uv.y & !mask_y) | (offset_y & mask_y);

    uv
  }

  fn get_texture(&mut self, uv: Coordinates2d, clut: Coordinates2d) -> Option<RgbColor> {
    match self.stat.texture_colors {
      TextureColors::FourBit => self.read_4bit_clut(uv, clut),
      TextureColors::EightBit => self.read_8bit_clut(uv, clut),
      TextureColors::FifteenBit => self.read_texture(uv)
    }
  }

  fn read_4bit_clut(&mut self, uv: Coordinates2d, clut: Coordinates2d) -> Option<RgbColor> {
    let tex_x_base = (self.stat.texture_x_base as i32) * 64;
    let tex_y_base = (self.stat.texture_y_base1 as i32) * 16;

    // since each pixel is 4 bits wide, we divide x offset by 2
    let offset_x = (2 * tex_x_base + (uv.x / 2)) as u32;
    let offset_y = (tex_y_base + uv.y) as u32;

    let texture_address = (offset_x + 2048 * offset_y) as usize;

    let block = (((uv.y / 64) * 4) + (uv.x / 64)) as isize;

    // each cacheline is organized in blocks of 4 * 64 cache lines.
    // each line is thus made up of 4 blocks,
    // this is why we multiply y by 4. since each cache entry is 16 4bpp pixels wide,
    // divide x by 16 to get the entry number, then add to y * 4 as described above.

    // see the diagram here for a good visual example:
    // https://psx-spx.consoledev.net/graphicsprocessingunitgpu/#gpu-texture-caching
    let entry = (((uv.y * 4) | ((uv.x / 16) & 0x3)) & 0xff) as usize;

    // each cache entry is 8 bytes wide
    let index = ((uv.x / 2) & 7) as usize;

    let cache_entry = &mut self.texture_cache[entry];

    // a cache entry can only have one block cached at a time
    if cache_entry.tag != block {
      for i in 0..8 {
        cache_entry.data[i] = self.vram[(texture_address & !0x7) + i];
      }

      cache_entry.tag = block;
    }

    let mut clut_entry = cache_entry.data[index] as usize;

    // each pixel is 4 bits, if x is odd, then get the upper 4 bits, otherwise get the lower ones
    if (uv.x & 0b1) != 0 {
      clut_entry >>= 4;
    } else {
      clut_entry &= 0xf;
    }

    let clut_address = (2 * clut.x + 2048 * clut.y) as isize;

    if self.clut_tag != clut_address {
      for i in 0..16 {
        let address = (clut_address as usize) + 2 * i;
        self.clut_cache[i] = util::read_half(&self.vram, address);
      }

      self.clut_tag = clut_address;
    }

    let texture = self.clut_cache[clut_entry];

    if texture != 0 {
      Some(GPU::translate_15bit_to_24(texture))
    } else {
      None
    }
  }

  fn read_8bit_clut(&mut self, uv: Coordinates2d, clut: Coordinates2d) -> Option<RgbColor> {
    let tex_x_base = (self.stat.texture_x_base as i32) * 64;
    let tex_y_base = (self.stat.texture_y_base1 as i32) * 16;

    let offset_x = (2 * tex_x_base + uv.x) as u32;
    let offset_y = (tex_y_base + uv.y) as u32;

    let texture_address = (offset_x + offset_y * 2048) as usize;

    // in this case, each cache line is organized in blocks of 8 * 32 cache lines,
    // and each cache entry is 8 8bpp pixels wide (half as many as 4bpp mode)
    let entry = ((4 * uv.y + ((uv.x / 8) & 0x7)) & 0xff) as usize;
    let block = ((uv.x / 32) + (uv.y / 64) * 8) as isize;

    let cache_entry = &mut self.texture_cache[entry];

    if cache_entry.tag != block {
      for i in 0..8 {
        cache_entry.data[i] = self.vram[(texture_address & !0x7) + i];
      }

      cache_entry.tag = block;
    }

    let index = (uv.x & 0x7) as usize;

    let clut_entry = cache_entry.data[index];

    let clut_address = (2 * clut.x + clut.y * 2048) as usize;

    if self.clut_tag != clut_address as isize {
      for i in 0..256 {
        let address = clut_address + 2 * i;

        self.clut_cache[i] = util::read_half(&self.vram, address);
      }

      self.clut_tag = clut_address as isize;
    }

    let texture = self.clut_cache[clut_entry as usize];

    if texture != 0 {
      Some(GPU::translate_15bit_to_24(texture))
    } else {
      None
    }
  }

  fn dither(&mut self, position: Coordinates2d, pixel: &mut RgbColor) {
    let x = (position.x & 3) as usize;
    let y = (position.y & 3) as usize;

    pixel.r = self.dither_table[x][y][pixel.r as usize];
    pixel.g = self.dither_table[x][y][pixel.g as usize];
    pixel.b = self.dither_table[x][y][pixel.b as usize];
  }


  fn read_texture(&mut self, uv: Coordinates2d) -> Option<RgbColor> {
    let tex_x_base = (self.stat.texture_x_base as i32) * 64;
    let tex_y_base = (self.stat.texture_y_base1 as i32) * 16;

    let offset_x = tex_x_base + uv.x;
    let offset_y = tex_y_base + uv.y;

    let texture_address = 2 * (offset_x + offset_y * 1024) as usize;

    // for this case, each cache entry is 8 * 32 cache lines, and each cache entry is 4 16bpp pixels wide
    let entry = (((uv.y * 8) + ((uv.x / 4 ) & 0x7)) & 0xff) as usize;
    let block = ((offset_x / 32) + (offset_y / 32) * 8) as isize;

    let cache_entry = &mut self.texture_cache[entry];

    if cache_entry.tag != block {
      for i in 0..8 {
        cache_entry.data[i] = self.vram[(texture_address & !0x7) + i];
      }
    }

    let index = ((uv.x * 2) & 0x7) as usize;

    let texture = (cache_entry.data[index] as u16) | (cache_entry.data[index + 1] as u16) << 8;

    if texture != 0 {
      Some(GPU::translate_15bit_to_24(texture))
    } else {
      None
    }
  }
}