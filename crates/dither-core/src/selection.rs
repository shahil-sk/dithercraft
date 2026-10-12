//! Interactive Selection Tools & Vector Marquee System.
//!
//! Provides pixel-precise geometric selection marquees (rectangular, elliptical),
//! polygonal lasso paths, OKLab color tolerance magic wand flood filling,
//! boolean selection arithmetic, and 2D contour segment extraction for animated marching ants.

use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::mask::MaskBuffer;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Boolean combination mode for updating active selections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionMode {
    New,
    Add,
    Subtract,
    Intersect,
}

/// Dynamic raster selection buffer backed by an 8-bit binary thresholded mask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionBuffer {
    pub width: u32,
    pub height: u32,
    pub mask: MaskBuffer,
}

impl SelectionBuffer {
    /// Create a new empty selection buffer.
    pub fn new(width: u32, height: u32) -> Result<Self, DitherError> {
        let mask = MaskBuffer::new(width, height)?;
        Ok(Self { width, height, mask })
    }

    /// Check if the selection has any active pixels.
    pub fn is_empty(&self) -> bool {
        self.mask.is_empty()
    }

    /// Clear all selected pixels.
    pub fn clear(&mut self) {
        self.mask.clear();
    }

    /// Select entire canvas.
    pub fn select_all(&mut self) {
        self.mask.fill_all();
    }

    /// Invert current selection.
    pub fn invert(&mut self) {
        self.mask.invert();
    }

    /// Extract minimal bounding box containing all selected pixels `(min_x, min_y, width, height)`.
    pub fn bounding_box(&self) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = u32::MAX;
        let mut max_x = 0;
        let mut min_y = u32::MAX;
        let mut max_y = 0;
        let mut has_any = false;

        let w = self.width;
        let h = self.height;

        for y in 0..h {
            let row = &self.mask.data[(y * w) as usize..((y + 1) * w) as usize];
            for x in 0..w {
                if row[x as usize] > 127 {
                    has_any = true;
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }

        if has_any { Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)) } else { None }
    }

    fn combine(&mut self, incoming: &[u8], mode: SelectionMode) {
        let len = self.mask.data.len().min(incoming.len());
        match mode {
            SelectionMode::New => {
                self.mask.data[..len].copy_from_slice(&incoming[..len]);
            }
            SelectionMode::Add => {
                for (dst, &src) in self.mask.data[..len].iter_mut().zip(&incoming[..len]) {
                    *dst = (*dst).max(src);
                }
            }
            SelectionMode::Subtract => {
                for (dst, &src) in self.mask.data[..len].iter_mut().zip(&incoming[..len]) {
                    if src > 127 {
                        *dst = 0;
                    }
                }
            }
            SelectionMode::Intersect => {
                for (dst, &src) in self.mask.data[..len].iter_mut().zip(&incoming[..len]) {
                    if *dst > 127 && src > 127 {
                        *dst = 255;
                    } else {
                        *dst = 0;
                    }
                }
            }
        }
    }

    /// Select a rectangular bounding area.
    pub fn select_rect(&mut self, x: u32, y: u32, w: u32, h: u32, mode: SelectionMode) {
        let total = (self.width as usize) * (self.height as usize);
        let mut incoming = vec![0u8; total];

        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);

        for py in y..max_y {
            let row_offset = (py as usize) * (self.width as usize);
            for px in x..max_x {
                incoming[row_offset + (px as usize)] = 255;
            }
        }

        self.combine(&incoming, mode);
    }

    /// Select an elliptical region centered at `(cx, cy)` with radii `(rx, ry)`.
    pub fn select_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, mode: SelectionMode) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let total = (self.width as usize) * (self.height as usize);
        let mut incoming = vec![0u8; total];

        let min_x = (cx - rx).floor().max(0.0) as u32;
        let max_x = (cx + rx).ceil().min(self.width as f32 - 1.0) as u32;
        let min_y = (cy - ry).floor().max(0.0) as u32;
        let max_y = (cy + ry).ceil().min(self.height as f32 - 1.0) as u32;

        let rx2 = rx * rx;
        let ry2 = ry * ry;

        for py in min_y..=max_y {
            let dy = py as f32 - cy;
            let norm_y = (dy * dy) / ry2;
            let row_offset = (py as usize) * (self.width as usize);

            for px in min_x..=max_x {
                let dx = px as f32 - cx;
                let norm_x = (dx * dx) / rx2;

                if norm_x + norm_y <= 1.0 {
                    incoming[row_offset + (px as usize)] = 255;
                }
            }
        }

        self.combine(&incoming, mode);
    }

    /// Select an arbitrary closed polygonal shape using point-in-polygon ray casting.
    pub fn select_polygon(&mut self, vertices: &[(f32, f32)], mode: SelectionMode) {
        if vertices.len() < 3 {
            return;
        }

        let total = (self.width as usize) * (self.height as usize);
        let mut incoming = vec![0u8; total];

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for &(vx, vy) in vertices {
            min_x = min_x.min(vx);
            max_x = max_x.max(vx);
            min_y = min_y.min(vy);
            max_y = max_y.max(vy);
        }

        let start_x = min_x.floor().max(0.0) as u32;
        let end_x = max_x.ceil().min(self.width as f32 - 1.0) as u32;
        let start_y = min_y.floor().max(0.0) as u32;
        let end_y = max_y.ceil().min(self.height as f32 - 1.0) as u32;

        let n = vertices.len();

        for py in start_y..=end_y {
            let y = py as f32 + 0.5;
            let row_offset = (py as usize) * (self.width as usize);

            for px in start_x..=end_x {
                let x = px as f32 + 0.5;
                let mut inside = false;

                let mut j = n - 1;
                for i in 0..n {
                    let (xi, yi) = vertices[i];
                    let (xj, yj) = vertices[j];

                    let intersect = ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
                    if intersect {
                        inside = !inside;
                    }
                    j = i;
                }

                if inside {
                    incoming[row_offset + (px as usize)] = 255;
                }
            }
        }

        self.combine(&incoming, mode);
    }

    /// Magic wand flood-fill selection: selects connected pixels whose OKLab distance
    /// from the seed pixel color is within `tolerance`.
    pub fn magic_wand(&mut self, input: &ImageBuffer, seed_x: u32, seed_y: u32, tolerance: f32, mode: SelectionMode) {
        if seed_x >= input.width || seed_y >= input.height {
            return;
        }

        let seed_color = input.get_pixel(seed_x, seed_y).unwrap_or(RgbColor::BLACK);
        let seed_lab = seed_color.to_oklab();
        let tol = tolerance.max(0.001);

        let w = input.width as usize;
        let h = input.height as usize;
        let total = w * h;

        let mut incoming = vec![0u8; total];
        let mut visited = vec![false; total];
        let mut queue = VecDeque::new();

        let start_idx = (seed_y as usize) * w + (seed_x as usize);
        visited[start_idx] = true;
        incoming[start_idx] = 255;
        queue.push_back((seed_x as usize, seed_y as usize));

        while let Some((cx, cy)) = queue.pop_front() {
            let neighbors = [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)];

            for (nx, ny) in neighbors {
                if nx < w && ny < h {
                    let idx = ny * w + nx;
                    if !visited[idx] {
                        visited[idx] = true;
                        let px = input.get_pixel(nx as u32, ny as u32).unwrap_or(RgbColor::BLACK);
                        let lab = px.to_oklab();

                        let dl = lab.l - seed_lab.l;
                        let da = lab.a - seed_lab.a;
                        let db = lab.b - seed_lab.b;
                        let dist = (dl * dl + da * da + db * db).sqrt();

                        if dist <= tol {
                            incoming[idx] = 255;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
        }

        self.combine(&incoming, mode);
    }

    /// Extract boundary edges as line segments for high-performance marching ants rendering.
    /// Each segment is a pair of points `((x0, y0), (x1, y1))` in pixel canvas coordinates.
    pub fn extract_marching_ants_segments(&self) -> Vec<((f32, f32), (f32, f32))> {
        let mut segments = Vec::new();
        let w = self.width as usize;
        let h = self.height as usize;

        for y in 0..h {
            let row_offset = y * w;
            for x in 0..w {
                let is_sel = self.mask.data[row_offset + x] > 127;
                if is_sel {
                    let fx = x as f32;
                    let fy = y as f32;

                    // Top edge check
                    if y == 0 || self.mask.data[row_offset - w + x] <= 127 {
                        segments.push(((fx, fy), (fx + 1.0, fy)));
                    }
                    // Bottom edge check
                    if y + 1 == h || self.mask.data[row_offset + w + x] <= 127 {
                        segments.push(((fx, fy + 1.0), (fx + 1.0, fy + 1.0)));
                    }
                    // Left edge check
                    if x == 0 || self.mask.data[row_offset + x - 1] <= 127 {
                        segments.push(((fx, fy), (fx, fy + 1.0)));
                    }
                    // Right edge check
                    if x + 1 == w || self.mask.data[row_offset + x + 1] <= 127 {
                        segments.push(((fx + 1.0, fy), (fx + 1.0, fy + 1.0)));
                    }
                }
            }
        }

        segments
    }

    /// Convert selection buffer directly into an 8-bit MaskBuffer.
    pub fn to_mask(&self) -> MaskBuffer {
        self.mask.clone()
    }

    /// Create selection buffer from an existing MaskBuffer.
    pub fn from_mask(mask: MaskBuffer) -> Self {
        Self { width: mask.width, height: mask.height, mask }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_rect_and_boolean_ops() {
        let mut sel = SelectionBuffer::new(10, 10).expect("sel");
        assert!(sel.is_empty());

        // Select 4x4 rect in top-left
        sel.select_rect(0, 0, 4, 4, SelectionMode::New);
        assert!(!sel.is_empty());
        assert_eq!(sel.bounding_box(), Some((0, 0, 4, 4)));

        // Add 2x2 rect
        sel.select_rect(4, 0, 2, 2, SelectionMode::Add);
        assert_eq!(sel.bounding_box(), Some((0, 0, 6, 4)));

        // Subtract 2x2 from center
        sel.select_rect(2, 2, 2, 2, SelectionMode::Subtract);
        assert_eq!(sel.mask.get_pixel(2, 2), Some(0));

        sel.clear();
        assert!(sel.is_empty());
    }

    #[test]
    fn test_selection_ellipse_and_polygon() {
        let mut sel = SelectionBuffer::new(20, 20).expect("sel");
        sel.select_ellipse(10.0, 10.0, 5.0, 5.0, SelectionMode::New);
        assert!(!sel.is_empty());
        assert_eq!(sel.mask.get_pixel(10, 10), Some(255));
        assert_eq!(sel.mask.get_pixel(0, 0), Some(0));

        let poly = vec![(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)];
        sel.select_polygon(&poly, SelectionMode::New);
        assert_eq!(sel.mask.get_pixel(5, 5), Some(255));
        assert_eq!(sel.mask.get_pixel(1, 1), Some(0));
    }

    #[test]
    fn test_marching_ants_segment_extraction() {
        let mut sel = SelectionBuffer::new(4, 4).expect("sel");
        sel.select_rect(1, 1, 2, 2, SelectionMode::New);
        let segs = sel.extract_marching_ants_segments();
        // A 2x2 square has 8 boundary segments (2 top, 2 bottom, 2 left, 2 right)
        assert_eq!(segs.len(), 8);
    }
}
