use super::{Coordinates2d, GPU, Vertex};

pub struct TextureDeltas {
  pub dudx: f32,
  pub dudy: f32,
  pub dvdx: f32,
  pub dvdy: f32
}

impl TextureDeltas {
  pub fn new(dudx: f32, dudy: f32, dvdx: f32, dvdy: f32) -> Self {
    Self {
      dudx,
      dudy,
      dvdx,
      dvdy
    }
  }

  pub fn get_texture_deltas(v: &[Vertex], cross_product: i32) -> Self {
    let dudx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].uv.x, v[0].p.y),
      Coordinates2d::new(v[1].uv.x, v[1].p.y),
      Coordinates2d::new(v[2].uv.x, v[2].p.y)
    );

    let dudy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].uv.x),
      Coordinates2d::new(v[1].p.x, v[1].uv.x),
      Coordinates2d::new(v[2].p.x, v[2].uv.x)
    );

    let dvdx_cp = GPU::cross_product(
      Coordinates2d::new(v[0].uv.y, v[0].p.y),
      Coordinates2d::new(v[1].uv.y, v[1].p.y),
      Coordinates2d::new(v[2].uv.y, v[2].p.y)
    );

    let dvdy_cp = GPU::cross_product(
      Coordinates2d::new(v[0].p.x, v[0].uv.y),
      Coordinates2d::new(v[1].p.x, v[1].uv.y),
      Coordinates2d::new(v[2].p.x, v[2].uv.y)
    );

    let dudx = dudx_cp as f32 / cross_product as f32;
    let dudy = dudy_cp as f32 / cross_product as f32;

    let dvdx = dvdx_cp as f32 / cross_product as f32;
    let dvdy = dvdy_cp as f32 / cross_product as f32;

    Self::new(dudx, dudy, dvdx, dvdy)
  }
}

pub struct ColorDeltas {
  pub drdx: f32,
  pub drdy: f32,
  pub dgdx: f32,
  pub dgdy: f32,
  pub dbdx: f32,
  pub dbdy: f32
}

impl ColorDeltas {
  pub fn new(drdx: f32, drdy: f32, dgdx: f32, dgdy: f32, dbdx: f32, dbdy: f32) -> Self {
    Self {
      drdx,
      drdy,
      dbdx,
      dbdy,
      dgdx,
      dgdy
    }
  }

  pub fn get_color_deltas(v: &[Vertex], cross_product: i32) -> ColorDeltas {
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


    let drdx = drdx_cp as f32 / cross_product as f32;
    let drdy = drdy_cp as f32 / cross_product as f32;

    let dgdx = dgdx_cp as f32 / cross_product as f32;
    let dgdy = dgdy_cp as f32 / cross_product as f32;

    let dbdx = dbdx_cp as f32 / cross_product as f32;
    let dbdy = dbdy_cp as f32 / cross_product as f32;

    Self::new(drdx, drdy, dgdx, dgdy, dbdx, dbdy)
  }


}
