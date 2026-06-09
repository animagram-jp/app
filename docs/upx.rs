// ---- Upx: 座標系 ----

pub struct Upx {
    pub origin_x:  f64,
    pub origin_y:  f64,
    pub x_unit_px: f64,
    pub y_unit_px: f64,
}

impl Upx {
    pub fn new(origin_x: f64, origin_y: f64, x_unit_px: f64, y_unit_px: f64) -> Self {
        Self { origin_x, origin_y, x_unit_px, y_unit_px }
    }

    pub fn update(&mut self, origin_x: f64, origin_y: f64, x_unit_px: f64) {
        self.origin_x  = origin_x;
        self.origin_y  = origin_y;
        self.x_unit_px = x_unit_px;
    }

    pub fn judge(&self, elements: &[UpxBlock], coord_x: f64, coord_y: f64) -> Vec<u8> {
        let cx = coord_x - self.origin_x;
        let cy = coord_y - self.origin_y;
        elements.iter().map(|el| {
            if cx >= el.ux1 && cx <= el.ux2 && cy >= el.uy1 && cy <= el.uy2 { 1 } else { 0 }
        }).collect()
    }
}

// ---- UpxBlock: 座標上の矩形 ----

pub struct UpxBlock {
    pub ux1: f64,
    pub uy1: f64,
    pub ux2: f64,
    pub uy2: f64,
    pub is_line: bool,
}

impl UpxBlock {
    // is_px=true:  x,y はviewport絶対px
    // is_px=false: x,y はupx単位の論理値
    pub fn new(u: &Upx, x: f64, y: f64, w: f64, h: f64, is_px: bool) -> Self {
        let x1 = if is_px { x - u.origin_x } else { x * u.x_unit_px };
        let y1 = if is_px { y - u.origin_y } else { y * u.y_unit_px };
        let x2 = x1 + w * u.x_unit_px;
        let y2 = y1 + h * u.y_unit_px;
        Self {
            ux1: x1,
            uy1: y1,
            ux2: x2,
            uy2: y2,
            is_line: x1 == x2 || y1 == y2,
        }
    }
}

// ---- Canvas: 境界を持つUpx空間 ----

pub struct Block {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub struct Canvas {
    pub x_count: f64,
    pub y_count: f64,
    memory:  Vec<Option<Block>>,
    free_ids: Vec<usize>,
}

impl Canvas {
    pub fn new(x_count: f64, y_count: f64) -> Self {
        Self { x_count, y_count, memory: Vec::new(), free_ids: Vec::new() }
    }

    pub fn add(&mut self, x: f64, y: f64, w: f64, h: f64) -> usize {
        let id = if let Some(id) = self.free_ids.pop() { id } else { self.memory.len() };
        let block = Block { x, y, w, h };
        if id < self.memory.len() {
            self.memory[id] = Some(block);
        } else {
            self.memory.push(Some(block));
        }
        id
    }

    pub fn update(&mut self, u: &Upx, id: usize, x: Option<f64>, y: Option<f64>, w: Option<f64>, h: Option<f64>) -> bool {
        let mem = match self.memory.get(id).and_then(|m| m.as_ref()) {
            Some(m) => m,
            None => return false,
        };
        let nx = x.unwrap_or(mem.x);
        let ny = y.unwrap_or(mem.y);
        let nw = w.unwrap_or(mem.w);
        let nh = h.unwrap_or(mem.h);
        let el     = UpxBlock::new(u, nx, ny, nw, nh, false);
        let bounds = UpxBlock::new(u, 0.0, 0.0, self.x_count, self.y_count, false);
        let in_bounds = el.ux1 >= bounds.ux1 && el.uy1 >= bounds.uy1
            && el.ux2 <= bounds.ux2 && el.uy2 <= bounds.uy2;
        if in_bounds {
            self.memory[id] = Some(Block { x: nx, y: ny, w: nw, h: nh });
        }
        in_bounds
    }

    pub fn remove(&mut self, id: usize) {
        if id < self.memory.len() && self.memory[id].is_some() {
            self.memory[id] = None;
            self.free_ids.push(id);
        }
    }

    pub fn judge(&self, u: &Upx, client_x: f64, client_y: f64) -> Option<usize> {
        let entries: Vec<(usize, &Block)> = self.memory.iter().enumerate()
            .filter_map(|(i, m)| m.as_ref().map(|b| (i, b)))
            .collect();
        let blocks: Vec<UpxBlock> = entries.iter()
            .map(|(_, b)| UpxBlock::new(u, b.x, b.y, b.w, b.h, false))
            .collect();
        let hits = u.judge(&blocks, client_x, client_y);
        hits.iter().rposition(|&h| h == 1).map(|i| entries[i].0)
    }

    pub fn snap(&self, u: &Upx, client_x: f64, client_y: f64) -> (f64, f64) {
        let el = UpxBlock::new(u, client_x, client_y, 1.0, 1.0, true);
        (
            (el.ux1 / u.x_unit_px).floor(),
            (el.uy1 / u.y_unit_px).floor(),
        )
    }

    pub fn get(&self, id: usize) -> Option<&Block> {
        self.memory.get(id).and_then(|m| m.as_ref())
    }
}
