use crate::algorithms::traits::DitherAlgorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::error::DitherError;
use crate::palette::Palette;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Geometric screening dot shape models for offset lithography halftoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HalftoneDotShape {
    #[default]
    EuclideanRound,
    Elliptical,
    Diamond,
    Line,
}

impl HalftoneDotShape {
    pub const ALL: [Self; 4] = [Self::EuclideanRound, Self::Elliptical, Self::Diamond, Self::Line];

    pub fn name(&self) -> &'static str {
        match self {
            Self::EuclideanRound => "Euclidean Round",
            Self::Elliptical => "Elliptical Dot",
            Self::Diamond => "Diamond Screen",
            Self::Line => "Linear Line Grating",
        }
    }
}

/// CMYK separation channel identifiers for four-color process printmaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmykChannel {
    Cyan,
    Magenta,
    Yellow,
    Black,
}

impl CmykChannel {
    pub const ALL: [Self; 4] = [Self::Cyan, Self::Magenta, Self::Yellow, Self::Black];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Cyan => "Cyan",
            Self::Magenta => "Magenta",
            Self::Yellow => "Yellow",
            Self::Black => "Black",
        }
    }

    pub fn ink_hex(&self) -> &'static str {
        match self {
            Self::Cyan => "#00FFFF",
            Self::Magenta => "#FF00FF",
            Self::Yellow => "#FFFF00",
            Self::Black => "#000000",
        }
    }
}

/// Four individual monochrome film positive separation buffers for physical printmaking.
/// In each separation buffer, 0 (black) represents ink deposition, and 255 (white) represents unprinted substrate.
#[derive(Debug, Clone)]
pub struct CmykSeparationPlates {
    pub cyan: ImageBuffer,
    pub magenta: ImageBuffer,
    pub yellow: ImageBuffer,
    pub black: ImageBuffer,
    pub width: u32,
    pub height: u32,
}

impl CmykSeparationPlates {
    pub fn get_channel(&self, channel: CmykChannel) -> &ImageBuffer {
        match channel {
            CmykChannel::Cyan => &self.cyan,
            CmykChannel::Magenta => &self.magenta,
            CmykChannel::Yellow => &self.yellow,
            CmykChannel::Black => &self.black,
        }
    }

    /// Export an SVG film positive plate for a specific CMYK channel.
    pub fn export_svg_plate(&self, channel: CmykChannel, angle_deg: f32, margin: f32, pixel_size: f32) -> Result<String, DitherError> {
        let buf = self.get_channel(channel);
        let width = self.width;
        let height = self.height;
        let art_w = width as f32 * pixel_size;
        let art_h = height as f32 * pixel_size;

        let header_h = 24.0;
        let footer_h = 24.0;
        let total_w = art_w + 2.0 * margin;
        let total_h = art_h + 2.0 * margin + header_h + footer_h;
        let art_x = margin;
        let art_y = margin + header_h;

        let mut svg = String::with_capacity(32 * 1024);
        svg.push_str(&format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total_w} {total_h}" width="{total_w}" height="{total_h}">"##
        ));
        svg.push('\n');

        // Registration Targets in 4 corners
        if margin >= 15.0 {
            let r = (margin * 0.35).min(8.0);
            let cross_len = r * 1.5;
            let corners = [
                (margin * 0.5, margin * 0.5),
                (total_w - margin * 0.5, margin * 0.5),
                (margin * 0.5, total_h - margin * 0.5),
                (total_w - margin * 0.5, total_h - margin * 0.5),
            ];

            svg.push_str("  <!-- Registration Targets -->\n");
            for (cx, cy) in corners {
                let x1 = cx - cross_len;
                let x2 = cx + cross_len;
                let y1 = cy - cross_len;
                let y2 = cy + cross_len;
                svg.push_str(&format!(
                    "  <g stroke=\"#000000\" stroke-width=\"0.75\" fill=\"none\">\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" />\n    <line x1=\"{x1}\" y1=\"{cy}\" x2=\"{x2}\" y2=\"{cy}\" />\n    <line x1=\"{cx}\" y1=\"{y1}\" x2=\"{cx}\" y2=\"{y2}\" />\n  </g>\n"
                ));
            }
        }

        // Print Slug Header
        let text_y = margin * 0.75;
        let plate_idx = match channel {
            CmykChannel::Cyan => 1,
            CmykChannel::Magenta => 2,
            CmykChannel::Yellow => 3,
            CmykChannel::Black => 4,
        };
        svg.push_str("  <!-- Print Slug Header -->\n");
        svg.push_str(&format!(
            "  <text x=\"{margin}\" y=\"{text_y}\" font-family=\"monospace, monospace\" font-size=\"9\" fill=\"#000000\">PLATE {}/4: {} ({:.1}° ROSETTE SCREEN) | DitherCraft Process Separation</text>\n",
            plate_idx,
            channel.name().to_uppercase(),
            angle_deg
        ));

        // Film Positive Geometry (100% Solid Black where ink is deposited)
        svg.push_str(&format!("  <!-- Process Film Positive: {} -->\n  <g fill=\"#000000\" stroke=\"none\">\n", channel.name()));

        for y in 0..height {
            let mut span_start: Option<u32> = None;
            for x in 0..width {
                // In separation positive, pixel is dark (0) where ink goes
                let px = buf.get_pixel(x, y).unwrap_or(RgbColor::WHITE);
                let is_ink = px.r == 0;

                if is_ink {
                    if span_start.is_none() {
                        span_start = Some(x);
                    }
                } else if let Some(start_x) = span_start {
                    let span_len = x - start_x;
                    let rx = art_x + (start_x as f32) * pixel_size;
                    let ry = art_y + (y as f32) * pixel_size;
                    let rw = (span_len as f32) * pixel_size;
                    let rh = pixel_size;
                    svg.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                    svg.push('\n');
                    span_start = None;
                }
            }
            if let Some(start_x) = span_start {
                let span_len = width - start_x;
                let rx = art_x + (start_x as f32) * pixel_size;
                let ry = art_y + (y as f32) * pixel_size;
                let rw = (span_len as f32) * pixel_size;
                let rh = pixel_size;
                svg.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                svg.push('\n');
            }
        }
        svg.push_str("  </g>\n");

        // Calibration Step Wedge (5-step 20%..100% opacity)
        let wedge_y = art_y + art_h + 10.0;
        let step_w = (art_w / 5.0).min(20.0);
        let step_h = 10.0;
        let opacities = [0.2, 0.4, 0.6, 0.8, 1.0];
        let start_x = art_x;
        svg.push_str("  <!-- Density Calibration Step Wedge -->\n  <g stroke=\"#000000\" stroke-width=\"0.5\">\n");
        for (i, &op) in opacities.iter().enumerate() {
            let sx = start_x + (i as f32) * step_w;
            svg.push_str(&format!(
                "    <rect x=\"{sx}\" y=\"{wedge_y}\" width=\"{step_w}\" height=\"{step_h}\" fill=\"#000000\" fill-opacity=\"{op}\" />\n"
            ));
        }
        svg.push_str("  </g>\n</svg>\n");

        Ok(svg)
    }

    /// Export all 4 CMYK process plates as individual SVGs with filenames.
    pub fn export_all_svg_plates(&self, halftoner: &CmykHalftone, margin: f32, pixel_size: f32) -> Result<Vec<(String, String)>, DitherError> {
        let channels = [
            (CmykChannel::Cyan, halftoner.cyan_angle_deg, "plate_cmyk_01_cyan.svg"),
            (CmykChannel::Magenta, halftoner.magenta_angle_deg, "plate_cmyk_02_magenta.svg"),
            (CmykChannel::Yellow, halftoner.yellow_angle_deg, "plate_cmyk_03_yellow.svg"),
            (CmykChannel::Black, halftoner.black_angle_deg, "plate_cmyk_04_black.svg"),
        ];

        let mut plates = Vec::with_capacity(4);
        for (channel, angle, filename) in channels {
            let svg = self.export_svg_plate(channel, angle, margin, pixel_size)?;
            plates.push((filename.to_string(), svg));
        }
        Ok(plates)
    }
}

/// CMYK Halftone Four-Color Process Separations with standard angled rosette screens,
/// configurable dot geometries, and Yule-Nielsen dot gain compensation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmykHalftone {
    pub frequency: f32,
    pub dot_shape: HalftoneDotShape,
    pub dot_gain: f32,
    pub cyan_angle_deg: f32,
    pub magenta_angle_deg: f32,
    pub yellow_angle_deg: f32,
    pub black_angle_deg: f32,
}

impl Default for CmykHalftone {
    fn default() -> Self {
        Self {
            frequency: 0.15,
            dot_shape: HalftoneDotShape::EuclideanRound,
            dot_gain: 0.0,
            cyan_angle_deg: 15.0,
            magenta_angle_deg: 75.0,
            yellow_angle_deg: 0.0,
            black_angle_deg: 45.0,
        }
    }
}

impl CmykHalftone {
    #[inline]
    fn screen_val(x: f32, y: f32, angle_rad: f32, freq: f32, shape: HalftoneDotShape) -> f32 {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let u = x * cos_a - y * sin_a;
        let v = x * sin_a + y * cos_a;

        match shape {
            HalftoneDotShape::EuclideanRound => {
                let sin_u = (u * freq * std::f32::consts::TAU).sin();
                let sin_v = (v * freq * std::f32::consts::TAU).sin();
                (sin_u * sin_v) * 0.5 + 0.5
            }
            HalftoneDotShape::Elliptical => {
                let cos_u = (u * freq * std::f32::consts::TAU).cos();
                let cos_v = (v * freq * std::f32::consts::TAU).cos();
                (cos_u * 0.65 + cos_v * 0.35) * 0.5 + 0.5
            }
            HalftoneDotShape::Diamond => {
                let u_norm = ((u * freq).rem_euclid(1.0) - 0.5).abs();
                let v_norm = ((v * freq).rem_euclid(1.0) - 0.5).abs();
                (1.0 - (u_norm + v_norm) * std::f32::consts::SQRT_2).clamp(0.0, 1.0)
            }
            HalftoneDotShape::Line => ((u * freq * std::f32::consts::TAU).cos()) * 0.5 + 0.5,
        }
    }

    #[inline]
    fn apply_dot_gain(coverage: f32, gain: f32) -> f32 {
        if gain <= 0.0 {
            coverage
        } else {
            // Yule-Nielsen parabolic physical dot spread peak at 50% tint
            (coverage + gain * 4.0 * coverage * (1.0 - coverage)).clamp(0.0, 1.0)
        }
    }

    /// Generate individual monochrome printable film positive separation plates.
    pub fn generate_separations(&self, input: &ImageBuffer) -> Result<CmykSeparationPlates, DitherError> {
        let width = input.width as usize;
        let height = input.height as usize;

        let mut cyan_buf = ImageBuffer::new(input.width, input.height)?;
        let mut magenta_buf = ImageBuffer::new(input.width, input.height)?;
        let mut yellow_buf = ImageBuffer::new(input.width, input.height)?;
        let mut black_buf = ImageBuffer::new(input.width, input.height)?;

        let c_rad = self.cyan_angle_deg.to_radians();
        let m_rad = self.magenta_angle_deg.to_radians();
        let y_rad = self.yellow_angle_deg.to_radians();
        let k_rad = self.black_angle_deg.to_radians();
        let freq = self.frequency;
        let shape = self.dot_shape;
        let gain = self.dot_gain;

        // Process all 4 channels
        let rows: Vec<_> = (0..height)
            .into_par_iter()
            .map(|py| {
                let y_f = py as f32;
                let row_in_start = py * width * 3;
                let mut c_row = vec![255u8; width * 3];
                let mut m_row = vec![255u8; width * 3];
                let mut y_row = vec![255u8; width * 3];
                let mut k_row = vec![255u8; width * 3];

                for px in 0..width {
                    let x_f = px as f32;
                    let in_idx = row_in_start + px * 3;

                    let r_norm = input.data[in_idx] as f32 / 255.0;
                    let g_norm = input.data[in_idx + 1] as f32 / 255.0;
                    let b_norm = input.data[in_idx + 2] as f32 / 255.0;

                    let max_rgb = r_norm.max(g_norm).max(b_norm);
                    let k = 1.0 - max_rgb;
                    let (c, m, y) = if k < 1.0 {
                        ((1.0 - r_norm - k) / (1.0 - k), (1.0 - g_norm - k) / (1.0 - k), (1.0 - b_norm - k) / (1.0 - k))
                    } else {
                        (0.0, 0.0, 0.0)
                    };

                    let c_gain = Self::apply_dot_gain(c, gain);
                    let m_gain = Self::apply_dot_gain(m, gain);
                    let y_gain = Self::apply_dot_gain(y, gain);
                    let k_gain = Self::apply_dot_gain(k, gain);

                    let c_is_dot = c_gain > Self::screen_val(x_f, y_f, c_rad, freq, shape);
                    let m_is_dot = m_gain > Self::screen_val(x_f, y_f, m_rad, freq, shape);
                    let y_is_dot = y_gain > Self::screen_val(x_f, y_f, y_rad, freq, shape);
                    let k_is_dot = k_gain > Self::screen_val(x_f, y_f, k_rad, freq, shape);

                    let out_idx = px * 3;
                    // Film positive: 0 = ink deposited (opaque black emulsion), 255 = clear
                    let c_val = if c_is_dot { 0 } else { 255 };
                    let m_val = if m_is_dot { 0 } else { 255 };
                    let y_val = if y_is_dot { 0 } else { 255 };
                    let k_val = if k_is_dot { 0 } else { 255 };

                    c_row[out_idx] = c_val;
                    c_row[out_idx + 1] = c_val;
                    c_row[out_idx + 2] = c_val;

                    m_row[out_idx] = m_val;
                    m_row[out_idx + 1] = m_val;
                    m_row[out_idx + 2] = m_val;

                    y_row[out_idx] = y_val;
                    y_row[out_idx + 1] = y_val;
                    y_row[out_idx + 2] = y_val;

                    k_row[out_idx] = k_val;
                    k_row[out_idx + 1] = k_val;
                    k_row[out_idx + 2] = k_val;
                }
                (py, c_row, m_row, y_row, k_row)
            })
            .collect();

        for (py, c_row, m_row, y_row, k_row) in rows {
            let row_start = py * width * 3;
            cyan_buf.data[row_start..row_start + width * 3].copy_from_slice(&c_row);
            magenta_buf.data[row_start..row_start + width * 3].copy_from_slice(&m_row);
            yellow_buf.data[row_start..row_start + width * 3].copy_from_slice(&y_row);
            black_buf.data[row_start..row_start + width * 3].copy_from_slice(&k_row);
        }

        Ok(CmykSeparationPlates {
            cyan: cyan_buf,
            magenta: magenta_buf,
            yellow: yellow_buf,
            black: black_buf,
            width: input.width,
            height: input.height,
        })
    }
}

impl DitherAlgorithm for CmykHalftone {
    fn name(&self) -> &'static str {
        "CMYK Halftone"
    }

    fn apply(&self, input: &ImageBuffer, palette: &Palette) -> Result<ImageBuffer, DitherError> {
        if palette.is_empty() {
            return Err(DitherError::EmptyPalette);
        }

        let seps = self.generate_separations(input)?;
        let mut out = ImageBuffer::new(input.width, input.height)?;
        let width = input.width as usize;

        out.data.par_chunks_exact_mut(width * 3).enumerate().for_each(|(py, row_out)| {
            for px in 0..width {
                let c_ink = seps.cyan.get_pixel(px as u32, py as u32).map(|p| p.r == 0).unwrap_or(false);
                let m_ink = seps.magenta.get_pixel(px as u32, py as u32).map(|p| p.r == 0).unwrap_or(false);
                let y_ink = seps.yellow.get_pixel(px as u32, py as u32).map(|p| p.r == 0).unwrap_or(false);
                let k_ink = seps.black.get_pixel(px as u32, py as u32).map(|p| p.r == 0).unwrap_or(false);

                let c_dot = if c_ink { 1.0f32 } else { 0.0f32 };
                let m_dot = if m_ink { 1.0f32 } else { 0.0f32 };
                let y_dot = if y_ink { 1.0f32 } else { 0.0f32 };
                let k_dot = if k_ink { 1.0f32 } else { 0.0f32 };

                // Recomposite CMYK to RGB
                let r_res = ((1.0 - c_dot) * (1.0 - k_dot) * 255.0) as u8;
                let g_res = ((1.0 - m_dot) * (1.0 - k_dot) * 255.0) as u8;
                let b_res = ((1.0 - y_dot) * (1.0 - k_dot) * 255.0) as u8;

                let quantized = palette.find_nearest(&RgbColor::new(r_res, g_res, b_res));
                let out_idx = px * 3;
                row_out[out_idx] = quantized.r;
                row_out[out_idx + 1] = quantized.g;
                row_out[out_idx + 2] = quantized.b;
            }
        });

        Ok(out)
    }
}
