use crate::buffer::ImageBuffer;
use crate::error::DitherError;
use crate::vector::rdp::{Point2D, rdp_simplify};
use serde::{Deserialize, Serialize};

/// Infill hatch style for pen plotter and laser fabrication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HatchStyle {
    /// Standard horizontal raster scanline strokes
    #[default]
    Horizontal,
    /// Vertical column strokes
    Vertical,
    /// Dual 45-degree diagonal cross-hatching
    CrossHatch45,
    /// Stippled dot peck-drilling / pen taps
    StipplePoisson,
}

/// Configuration parameters for CNC pen plotter and laser toolpath generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCodeConfig {
    pub scale_mm_per_pixel: f32,
    pub travel_feed_rate: f32,
    pub draw_feed_rate: f32,
    pub pen_up_z: f32,
    pub pen_down_z: f32,
    pub rdp_tolerance: f32,
    pub threshold: u8,
    pub arc_fitting: bool,
    pub arc_tolerance: f32,
    pub nib_width_mm: f32,
    pub bi_directional_offset: bool,
    pub tool_change_pause: bool,
    pub tool_number: Option<u8>,
    pub hatch_style: HatchStyle,
    pub hatch_spacing_mm: f32,
}

impl Default for GCodeConfig {
    fn default() -> Self {
        Self {
            scale_mm_per_pixel: 0.5,
            travel_feed_rate: 3000.0,
            draw_feed_rate: 1200.0,
            pen_up_z: 5.0,
            pen_down_z: 0.0,
            rdp_tolerance: 0.25,
            threshold: 128,
            arc_fitting: false,
            arc_tolerance: 0.1,
            nib_width_mm: 0.5,
            bi_directional_offset: false,
            tool_change_pause: false,
            tool_number: None,
            hatch_style: HatchStyle::Horizontal,
            hatch_spacing_mm: 0.5,
        }
    }
}

/// A continuous drawing stroke line segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolpathStroke {
    pub points: Vec<Point2D>,
}

#[derive(Debug)]
enum MotionSegment {
    Linear(Point2D),
    Arc { end: Point2D, i: f32, j: f32, cw: bool },
}

pub struct GCodeExporter;

impl GCodeExporter {
    /// Extract horizontal raster scanlines into toolpath strokes.
    pub fn extract_raster_strokes(input: &ImageBuffer, config: &GCodeConfig) -> Vec<ToolpathStroke> {
        let width = input.width;
        let height = input.height;
        let scale = config.scale_mm_per_pixel;
        let mut strokes = Vec::new();

        for y in 0..height {
            let mut span_start = None;
            for x in 0..width {
                let px = input.get_pixel(x, y).unwrap_or(crate::color::RgbColor::WHITE);
                let is_dark = px.luminance() <= config.threshold;
                if is_dark {
                    if span_start.is_none() {
                        span_start = Some(x);
                    }
                } else if let Some(start_x) = span_start {
                    let p1 = Point2D::new(start_x as f32 * scale, y as f32 * scale);
                    let p2 = Point2D::new(x as f32 * scale, y as f32 * scale);
                    strokes.push(ToolpathStroke { points: vec![p1, p2] });
                    span_start = None;
                }
            }
            if let Some(start_x) = span_start {
                let p1 = Point2D::new(start_x as f32 * scale, y as f32 * scale);
                let p2 = Point2D::new(width as f32 * scale, y as f32 * scale);
                strokes.push(ToolpathStroke { points: vec![p1, p2] });
            }
        }

        strokes
    }

    /// Extract strokes matching the configured hatch infill style.
    pub fn extract_hatch_strokes(input: &ImageBuffer, config: &GCodeConfig) -> Vec<ToolpathStroke> {
        match config.hatch_style {
            HatchStyle::Horizontal => Self::extract_raster_strokes(input, config),
            HatchStyle::Vertical => {
                let width = input.width;
                let height = input.height;
                let scale = config.scale_mm_per_pixel;
                let mut strokes = Vec::new();

                for x in 0..width {
                    let mut span_start = None;
                    for y in 0..height {
                        let px = input.get_pixel(x, y).unwrap_or(crate::color::RgbColor::WHITE);
                        let is_dark = px.luminance() <= config.threshold;
                        if is_dark {
                            if span_start.is_none() {
                                span_start = Some(y);
                            }
                        } else if let Some(start_y) = span_start {
                            let p1 = Point2D::new(x as f32 * scale, start_y as f32 * scale);
                            let p2 = Point2D::new(x as f32 * scale, y as f32 * scale);
                            strokes.push(ToolpathStroke { points: vec![p1, p2] });
                            span_start = None;
                        }
                    }
                    if let Some(start_y) = span_start {
                        let p1 = Point2D::new(x as f32 * scale, start_y as f32 * scale);
                        let p2 = Point2D::new(x as f32 * scale, height as f32 * scale);
                        strokes.push(ToolpathStroke { points: vec![p1, p2] });
                    }
                }
                strokes
            }
            HatchStyle::CrossHatch45 => {
                let mut strokes = Self::extract_raster_strokes(input, config);
                let width = input.width as i32;
                let height = input.height as i32;
                let scale = config.scale_mm_per_pixel;
                let step = ((config.hatch_spacing_mm / config.scale_mm_per_pixel).round() as i32).max(1);

                // +45 degree diagonals (x + y = k)
                let max_sum = width + height;
                for k in (0..max_sum).step_by(step as usize) {
                    let mut span_start = None;
                    for x in 0..width {
                        let y = k - x;
                        if y >= 0 && y < height {
                            let px = input.get_pixel(x as u32, y as u32).unwrap_or(crate::color::RgbColor::WHITE);
                            let is_dark = px.luminance() <= config.threshold;
                            if is_dark {
                                if span_start.is_none() {
                                    span_start = Some((x, y));
                                }
                            } else if let Some((sx, sy)) = span_start {
                                let p1 = Point2D::new(sx as f32 * scale, sy as f32 * scale);
                                let p2 = Point2D::new(x as f32 * scale, y as f32 * scale);
                                strokes.push(ToolpathStroke { points: vec![p1, p2] });
                                span_start = None;
                            }
                        }
                    }
                }
                strokes
            }
            HatchStyle::StipplePoisson => {
                let width = input.width;
                let height = input.height;
                let scale = config.scale_mm_per_pixel;
                let step = ((config.hatch_spacing_mm / config.scale_mm_per_pixel).round() as u32).max(1);
                let mut strokes = Vec::new();

                for y in (0..height).step_by(step as usize) {
                    for x in (0..width).step_by(step as usize) {
                        let px = input.get_pixel(x, y).unwrap_or(crate::color::RgbColor::WHITE);
                        if px.luminance() <= config.threshold {
                            let pt = Point2D::new(x as f32 * scale, y as f32 * scale);
                            strokes.push(ToolpathStroke { points: vec![pt] });
                        }
                    }
                }
                strokes
            }
        }
    }

    /// Sort strokes using Nearest-Neighbor heuristic to minimize rapid travel distance.
    pub fn optimize_toolpath_order(strokes: &mut [ToolpathStroke]) {
        if strokes.len() <= 2 {
            return;
        }

        let mut current_pos = Point2D::new(0.0, 0.0);
        let n = strokes.len();

        for i in 0..n {
            let mut best_idx = i;
            let mut min_dist = f32::MAX;
            let mut reverse_best = false;

            for (offset, stroke) in strokes[i..].iter().enumerate() {
                let j = i + offset;
                let start_pt = stroke.points.first().copied().unwrap_or(current_pos);
                let end_pt = stroke.points.last().copied().unwrap_or(current_pos);

                let d_start = current_pos.distance_to(&start_pt);
                let d_end = current_pos.distance_to(&end_pt);

                if d_start < min_dist {
                    min_dist = d_start;
                    best_idx = j;
                    reverse_best = false;
                }
                if d_end < min_dist {
                    min_dist = d_end;
                    best_idx = j;
                    reverse_best = true;
                }
            }

            strokes.swap(i, best_idx);
            if reverse_best {
                strokes[i].points.reverse();
            }
            if let Some(&last_pt) = strokes[i].points.last() {
                current_pos = last_pt;
            }
        }
    }

    /// Try to fit 3 points into a circular arc within tolerance.
    fn fit_circle_3pts(p1: Point2D, p2: Point2D, p3: Point2D) -> Option<(f32, f32, f32, bool)> {
        let d = 2.0 * (p1.x * (p2.y - p3.y) + p2.x * (p3.y - p1.y) + p3.x * (p1.y - p2.y));
        if d.abs() < 1e-5 {
            return None; // Collinear
        }

        let p1_sq = p1.x * p1.x + p1.y * p1.y;
        let p2_sq = p2.x * p2.x + p2.y * p2.y;
        let p3_sq = p3.x * p3.x + p3.y * p3.y;

        let cx = (p1_sq * (p2.y - p3.y) + p2_sq * (p3.y - p1.y) + p3_sq * (p1.y - p2.y)) / d;
        let cy = (p1_sq * (p3.x - p2.x) + p2_sq * (p1.x - p3.x) + p3_sq * (p2.x - p1.x)) / d;
        let r = ((p1.x - cx).powi(2) + (p1.y - cy).powi(2)).sqrt();

        if r < 0.2 || r > 5000.0 {
            return None;
        }

        // Cross product for clockwise (CW: z < 0) vs counter-clockwise (CCW: z > 0)
        let z = (p2.x - p1.x) * (p3.y - p2.y) - (p2.y - p1.y) * (p3.x - p2.x);
        let cw = z < 0.0;

        Some((cx, cy, r, cw))
    }

    /// Convert polyline into linear motions and G02/G03 circular arc motions.
    fn convert_stroke_to_motions(pts: &[Point2D], arc_fitting: bool, arc_tolerance: f32) -> Vec<MotionSegment> {
        if pts.len() < 2 {
            return Vec::new();
        }
        if !arc_fitting || pts.len() < 3 {
            return pts[1..].iter().map(|&p| MotionSegment::Linear(p)).collect();
        }

        let mut motions = Vec::new();
        let mut i = 0;

        while i + 1 < pts.len() {
            if i + 2 < pts.len()
                && let Some((cx, cy, r, cw)) = Self::fit_circle_3pts(pts[i], pts[i + 1], pts[i + 2])
            {
                // Verify intermediate point matches arc radius within tolerance
                let d_mid = ((pts[i + 1].x - cx).powi(2) + (pts[i + 1].y - cy).powi(2)).sqrt();
                if (d_mid - r).abs() <= arc_tolerance {
                    let end = pts[i + 2];
                    let rel_i = cx - pts[i].x;
                    let rel_j = cy - pts[i].y;
                    motions.push(MotionSegment::Arc { end, i: rel_i, j: rel_j, cw });
                    i += 2;
                    continue;
                }
            }

            motions.push(MotionSegment::Linear(pts[i + 1]));
            i += 1;
        }

        motions
    }

    /// Generate complete G-code program string from image buffer.
    pub fn generate_gcode(input: &ImageBuffer, config: &GCodeConfig) -> Result<String, DitherError> {
        let mut strokes = Self::extract_hatch_strokes(input, config);
        Self::optimize_toolpath_order(&mut strokes);

        // Bi-directional nib width endpoint compensation
        if config.bi_directional_offset && config.nib_width_mm > 0.0 {
            let offset = config.nib_width_mm * 0.25;
            for (idx, stroke) in strokes.iter_mut().enumerate() {
                if stroke.points.len() >= 2 && idx % 2 == 1 {
                    let first = &mut stroke.points[0];
                    first.x += offset;
                    let last = stroke.points.last_mut().expect("points non-empty");
                    last.x -= offset;
                }
            }
        }

        let mut gcode = String::with_capacity(32 * 1024);
        gcode.push_str("; ================================================\n");
        gcode.push_str("; Generated by DitherCraft — CNC / Plotter Toolpath\n");
        gcode.push_str(&format!("; Total strokes: {}\n", strokes.len()));
        gcode.push_str(&format!("; Hatch style: {:?}\n", config.hatch_style));
        gcode.push_str(&format!("; Arc fitting: {}\n", config.arc_fitting));
        gcode.push_str("; ================================================\n");
        gcode.push_str("G21 ; Units: mm\n");
        gcode.push_str("G90 ; Absolute positioning\n");

        if config.tool_change_pause {
            if let Some(tool) = config.tool_number {
                gcode.push_str(&format!("T{:02} M06 ; Tool change\n", tool));
            }
            gcode.push_str("M00 ; Pause for pen inspection\n");
        }

        gcode.push_str(&format!("G00 Z{:.3} F{:.1}\n", config.pen_up_z, config.travel_feed_rate));

        for stroke in &strokes {
            if stroke.points.is_empty() {
                continue;
            }

            let simplified = if config.rdp_tolerance > 0.0 { rdp_simplify(&stroke.points, config.rdp_tolerance) } else { stroke.points.clone() };

            let first = simplified[0];
            // Rapid travel to start of stroke
            gcode.push_str(&format!("G00 X{:.3} Y{:.3} F{:.1}\n", first.x, first.y, config.travel_feed_rate));
            // Pen down
            gcode.push_str(&format!("G01 Z{:.3} F{:.1}\n", config.pen_down_z, config.draw_feed_rate));

            // If stroke is a single point (e.g., stipple peck tap)
            if simplified.len() == 1 {
                gcode.push_str(&format!("G01 X{:.3} Y{:.3} F{:.1}\n", first.x, first.y, config.draw_feed_rate));
            } else {
                let motions = Self::convert_stroke_to_motions(&simplified, config.arc_fitting, config.arc_tolerance);
                for motion in motions {
                    match motion {
                        MotionSegment::Linear(pt) => {
                            gcode.push_str(&format!("G01 X{:.3} Y{:.3} F{:.1}\n", pt.x, pt.y, config.draw_feed_rate));
                        }
                        MotionSegment::Arc { end, i, j, cw } => {
                            let cmd = if cw { "G02" } else { "G03" };
                            gcode.push_str(&format!("{cmd} X{:.3} Y{:.3} I{:.3} J{:.3} F{:.1}\n", end.x, end.y, i, j, config.draw_feed_rate));
                        }
                    }
                }
            }

            // Pen up
            gcode.push_str(&format!("G00 Z{:.3} F{:.1}\n", config.pen_up_z, config.travel_feed_rate));
        }

        gcode.push_str("G00 X0.000 Y0.000 ; Return home\n");
        gcode.push_str("M02 ; End of program\n");

        Ok(gcode)
    }
}
