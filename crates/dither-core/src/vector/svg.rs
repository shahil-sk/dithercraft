use crate::buffer::ImageBuffer;
use crate::error::DitherError;

/// Black Vector Isolation: Extracts only the darkest pixel data as solid vector geometry paths
/// tailored for vinyl cutters, plotting devices, laser engravers, and embroidery machines.
#[derive(Debug, Clone)]
pub struct BlackVectorIsolation {
    pub max_luminance_threshold: u8,
    pub pixel_size: f32, // scale in SVG units (e.g. 1.0mm or 1.0px)
}

impl Default for BlackVectorIsolation {
    fn default() -> Self {
        Self { max_luminance_threshold: 128, pixel_size: 1.0 }
    }
}

impl BlackVectorIsolation {
    /// Trace black pixel runs into an optimized SVG string with merged horizontal spans.
    pub fn trace_to_svg(&self, input: &ImageBuffer) -> Result<String, DitherError> {
        let width = input.width;
        let height = input.height;
        let scale = self.pixel_size;
        let svg_w = (width as f32) * scale;
        let svg_h = (height as f32) * scale;

        let mut svg = String::with_capacity(32 * 1024);
        svg.push_str(&format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" width="{svg_w}" height="{svg_h}">"##));
        svg.push('\n');
        svg.push_str("  <g fill=\"#000000\" stroke=\"none\">\n");

        // Merge contiguous horizontal black pixels into single rectangle paths
        for y in 0..height {
            let mut span_start: Option<u32> = None;
            for x in 0..width {
                let px = input.get_pixel(x, y).unwrap_or(crate::color::RgbColor::WHITE);
                let is_dark = px.luminance() <= self.max_luminance_threshold;

                if is_dark {
                    if span_start.is_none() {
                        span_start = Some(x);
                    }
                } else if let Some(start_x) = span_start {
                    let span_len = x - start_x;
                    let rx = (start_x as f32) * scale;
                    let ry = (y as f32) * scale;
                    let rw = (span_len as f32) * scale;
                    let rh = scale;
                    svg.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                    svg.push('\n');
                    span_start = None;
                }
            }
            if let Some(start_x) = span_start {
                let span_len = width - start_x;
                let rx = (start_x as f32) * scale;
                let ry = (y as f32) * scale;
                let rw = (span_len as f32) * scale;
                let rh = scale;
                svg.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                svg.push('\n');
            }
        }

        svg.push_str("  </g>\n</svg>\n");
        Ok(svg)
    }
}

/// Multi-color SVG separation exporter.
/// Groups pixel spans by their nearest palette swatch into dedicated SVG layer groups:
/// `<g id="color-{hex}" fill="{hex}"> ... </g>`
#[derive(Debug, Clone)]
pub struct MultiColorSvgSeparation {
    pub pixel_size: f32,
    pub include_white: bool,
}

impl Default for MultiColorSvgSeparation {
    fn default() -> Self {
        Self { pixel_size: 1.0, include_white: false }
    }
}

impl MultiColorSvgSeparation {
    pub fn trace_palette_layers(&self, input: &ImageBuffer, palette: &crate::palette::Palette) -> Result<String, DitherError> {
        let width = input.width;
        let height = input.height;
        let scale = self.pixel_size;
        let svg_w = (width as f32) * scale;
        let svg_h = (height as f32) * scale;

        let mut svg = String::with_capacity(64 * 1024);
        svg.push_str(&format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" width="{svg_w}" height="{svg_h}">"##));
        svg.push('\n');

        for (swatch_idx, swatch) in palette.swatches.iter().enumerate() {
            let hex = swatch.color.to_hex();
            if !self.include_white && swatch.color == crate::color::RgbColor::WHITE {
                continue;
            }

            let mut group_content = String::new();
            for y in 0..height {
                let mut span_start: Option<u32> = None;
                for x in 0..width {
                    let px = input.get_pixel(x, y).unwrap_or(crate::color::RgbColor::WHITE);
                    let nearest_idx = palette.find_nearest_index(&px);

                    if nearest_idx == swatch_idx {
                        if span_start.is_none() {
                            span_start = Some(x);
                        }
                    } else if let Some(start_x) = span_start {
                        let span_len = x - start_x;
                        let rx = (start_x as f32) * scale;
                        let ry = (y as f32) * scale;
                        let rw = (span_len as f32) * scale;
                        let rh = scale;
                        group_content.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                        group_content.push('\n');
                        span_start = None;
                    }
                }
                if let Some(start_x) = span_start {
                    let span_len = width - start_x;
                    let rx = (start_x as f32) * scale;
                    let ry = (y as f32) * scale;
                    let rw = (span_len as f32) * scale;
                    let rh = scale;
                    group_content.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                    group_content.push('\n');
                }
            }

            if !group_content.is_empty() {
                svg.push_str(&format!(r#"  <g id="layer-{}-{}" fill="{}">"#, swatch_idx, hex.trim_start_matches('#'), hex));
                svg.push('\n');
                svg.push_str(&group_content);
                svg.push_str("  </g>\n");
            }
        }

        svg.push_str("</svg>\n");
        Ok(svg)
    }
}

/// Spot Color Plate Separation: Generates 100% solid black film positive vector separations
/// for silkscreen, risograph, lithographic printmakers, and stencil cutting.
/// Includes four corner registration crosshairs, exposure step calibration wedges, and print slug annotations.
#[derive(Debug, Clone)]
pub struct SpotColorPlateSeparation {
    pub pixel_size: f32,
    pub margin: f32,
    pub include_registration_marks: bool,
    pub include_calibration_wedge: bool,
    pub include_slug: bool,
}

impl Default for SpotColorPlateSeparation {
    fn default() -> Self {
        Self { pixel_size: 1.0, margin: 30.0, include_registration_marks: true, include_calibration_wedge: true, include_slug: true }
    }
}

impl SpotColorPlateSeparation {
    /// Generate an SVG film positive plate for a specific swatch index in the palette.
    pub fn generate_plate_svg(&self, input: &ImageBuffer, palette: &crate::palette::Palette, swatch_idx: usize) -> Result<String, DitherError> {
        if swatch_idx >= palette.swatches.len() {
            return Err(DitherError::InvalidParameter(format!("swatch index {swatch_idx} out of range [0, {})", palette.swatches.len())));
        }

        let swatch = &palette.swatches[swatch_idx];
        let hex = swatch.color.to_hex();

        let width = input.width;
        let height = input.height;
        let scale = self.pixel_size;
        let art_w = (width as f32) * scale;
        let art_h = (height as f32) * scale;
        let margin = self.margin;

        let header_h = if self.include_slug { 24.0 } else { 0.0 };
        let footer_h = if self.include_calibration_wedge { 24.0 } else { 0.0 };

        let total_w = art_w + 2.0 * margin;
        let total_h = art_h + 2.0 * margin + header_h + footer_h;
        let art_x = margin;
        let art_y = margin + header_h;

        let mut svg = String::with_capacity(32 * 1024);
        svg.push_str(&format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total_w} {total_h}" width="{total_w}" height="{total_h}">"##
        ));
        svg.push('\n');

        // Optional Registration Marks in 4 corners
        if self.include_registration_marks && margin >= 15.0 {
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
                svg.push_str(&format!(
                    "  <g stroke=\"#000000\" stroke-width=\"0.75\" fill=\"none\">\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" />\n    <line x1=\"{}\" y1=\"{cy}\" x2=\"{}\" y2=\"{cy}\" />\n    <line x1=\"{cx}\" y1=\"{}\" x2=\"{cx}\" y2=\"{}\" />\n  </g>\n",
                    cx - cross_len,
                    cx + cross_len,
                    cy - cross_len,
                    cy + cross_len
                ));
            }
        }

        // Optional Print Slug Header
        if self.include_slug {
            svg.push_str("  <!-- Print Slug Header -->\n");
            let text_y = margin * 0.75;
            svg.push_str(&format!(
                "  <text x=\"{margin}\" y=\"{text_y}\" font-family=\"monospace, monospace\" font-size=\"9\" fill=\"#000000\">PLATE {}/{}: {} | DitherCraft Spot Color Separation</text>\n",
                swatch_idx + 1,
                palette.swatches.len(),
                hex
            ));
        }

        // Artwork Film Positive (100% Solid Black)
        svg.push_str(&format!("  <!-- Plate Artwork: Color {} -->\n  <g fill=\"#000000\" stroke=\"none\">\n", hex));

        for y in 0..height {
            let mut span_start: Option<u32> = None;
            for x in 0..width {
                let px = input.get_pixel(x, y).unwrap_or(crate::color::RgbColor::WHITE);
                let nearest = palette.find_nearest_index(&px);

                if nearest == swatch_idx {
                    if span_start.is_none() {
                        span_start = Some(x);
                    }
                } else if let Some(start_x) = span_start {
                    let span_len = x - start_x;
                    let rx = art_x + (start_x as f32) * scale;
                    let ry = art_y + (y as f32) * scale;
                    let rw = (span_len as f32) * scale;
                    let rh = scale;
                    svg.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                    svg.push('\n');
                    span_start = None;
                }
            }
            if let Some(start_x) = span_start {
                let span_len = width - start_x;
                let rx = art_x + (start_x as f32) * scale;
                let ry = art_y + (y as f32) * scale;
                let rw = (span_len as f32) * scale;
                let rh = scale;
                svg.push_str(&format!(r#"    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}" />"#));
                svg.push('\n');
            }
        }
        svg.push_str("  </g>\n");

        // Optional Calibration Step Wedge
        if self.include_calibration_wedge {
            svg.push_str("  <!-- Density Calibration Step Wedge -->\n");
            let wedge_y = art_y + art_h + 10.0;
            let step_w = (art_w / 5.0).min(20.0);
            let step_h = 10.0;
            let opacities = [0.2, 0.4, 0.6, 0.8, 1.0];
            let start_x = art_x;

            svg.push_str("  <g stroke=\"#000000\" stroke-width=\"0.5\">\n");
            for (i, &op) in opacities.iter().enumerate() {
                let sx = start_x + (i as f32) * step_w;
                svg.push_str(&format!(
                    "    <rect x=\"{sx}\" y=\"{wedge_y}\" width=\"{step_w}\" height=\"{step_h}\" fill=\"#000000\" fill-opacity=\"{op}\" />\n"
                ));
            }
            svg.push_str("  </g>\n");
        }

        svg.push_str("</svg>\n");
        Ok(svg)
    }

    /// Generate film positive SVG separations for all colors in the palette.
    /// Returns a list of `(filename, svg_content)` tuples.
    pub fn generate_all_plates(&self, input: &ImageBuffer, palette: &crate::palette::Palette) -> Result<Vec<(String, String)>, DitherError> {
        let mut plates = Vec::with_capacity(palette.swatches.len());
        for (idx, swatch) in palette.swatches.iter().enumerate() {
            let svg = self.generate_plate_svg(input, palette, idx)?;
            let hex_clean = swatch.color.to_hex();
            let safe_name = hex_clean.trim_start_matches('#');
            let filename = format!("plate_{:02}_{}.svg", idx + 1, safe_name);
            plates.push((filename, svg));
        }
        Ok(plates)
    }
}

/// Geometric transformation parameters for SVG canvas projection.
#[derive(Debug, Clone, Copy)]
pub struct SvgTransform {
    pub scale_x: f32,
    pub scale_y: f32,
    pub off_x: f32,
    pub off_y: f32,
}

/// Pure-Rust SVG Rasterizer:
/// Decodes and rasterizes vector SVGs (viewBox, rects, circles, lines, polylines, polygons, paths)
/// into an ImageBuffer without any external C or graphics libraries.
#[derive(Debug, Clone)]
pub struct SvgRasterizer {
    pub default_width: u32,
    pub default_height: u32,
    pub background: crate::color::RgbColor,
}

impl Default for SvgRasterizer {
    fn default() -> Self {
        Self { default_width: 512, default_height: 512, background: crate::color::RgbColor::WHITE }
    }
}

impl SvgRasterizer {
    /// Create a new SvgRasterizer with target dimensions.
    pub fn new(default_width: u32, default_height: u32, background: crate::color::RgbColor) -> Self {
        Self { default_width: default_width.clamp(1, 4096), default_height: default_height.clamp(1, 4096), background }
    }

    /// Parse a CSS/SVG color value into RgbColor.
    pub fn parse_color(val: &str) -> Option<crate::color::RgbColor> {
        let s = val.trim().to_ascii_lowercase();
        if s.is_empty() || s == "none" || s == "transparent" {
            return None;
        }

        if let Some(hex) = s.strip_prefix('#') {
            match hex.len() {
                3 => {
                    let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                    let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                    let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                    return Some(crate::color::RgbColor::new(r, g, b));
                }
                6 => {
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    return Some(crate::color::RgbColor::new(r, g, b));
                }
                8 => {
                    // RGBA hex - use RGB components
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    return Some(crate::color::RgbColor::new(r, g, b));
                }
                _ => return None,
            }
        }

        if s.starts_with("rgb(") && s.ends_with(')') {
            let inner = &s[4..s.len() - 1];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() >= 3 {
                let r = parts[0].trim().parse::<u8>().ok()?;
                let g = parts[1].trim().parse::<u8>().ok()?;
                let b = parts[2].trim().parse::<u8>().ok()?;
                return Some(crate::color::RgbColor::new(r, g, b));
            }
        }

        match s.as_str() {
            "black" => Some(crate::color::RgbColor::BLACK),
            "white" => Some(crate::color::RgbColor::WHITE),
            "red" => Some(crate::color::RgbColor::new(255, 0, 0)),
            "green" => Some(crate::color::RgbColor::new(0, 128, 0)),
            "blue" => Some(crate::color::RgbColor::new(0, 0, 255)),
            "yellow" => Some(crate::color::RgbColor::new(255, 255, 0)),
            "cyan" => Some(crate::color::RgbColor::new(0, 255, 255)),
            "magenta" => Some(crate::color::RgbColor::new(255, 0, 255)),
            "gray" | "grey" => Some(crate::color::RgbColor::new(128, 128, 128)),
            "orange" => Some(crate::color::RgbColor::new(255, 165, 0)),
            "purple" => Some(crate::color::RgbColor::new(128, 0, 128)),
            _ => None,
        }
    }

    /// Extract an attribute value from an XML tag string.
    pub fn extract_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
        let pattern = format!("{}=\"", name);
        if let Some(pos) = tag.find(&pattern) {
            let start = pos + pattern.len();
            if let Some(end) = tag[start..].find('"') {
                return Some(&tag[start..start + end]);
            }
        }
        let pattern_single = format!("{}='", name);
        if let Some(pos) = tag.find(&pattern_single) {
            let start = pos + pattern_single.len();
            if let Some(end) = tag[start..].find('\'') {
                return Some(&tag[start..start + end]);
            }
        }
        None
    }

    /// Parse numeric coordinate or dimension attribute.
    fn parse_dim(val: &str) -> Option<f32> {
        let trimmed = val.trim().trim_end_matches("px").trim_end_matches("pt");
        trimmed.parse::<f32>().ok()
    }

    /// Rasterize an SVG document into an ImageBuffer.
    pub fn rasterize(&self, svg_text: &str) -> Result<ImageBuffer, DitherError> {
        let (width, height, vb_min_x, vb_min_y, vb_w, vb_h) = {
            let mut w = self.default_width as f32;
            let mut h = self.default_height as f32;
            let mut min_x = 0.0_f32;
            let mut min_y = 0.0_f32;
            let mut v_w = w;
            let mut v_h = h;

            if let Some(svg_start) = svg_text.find("<svg") {
                let tag_end = svg_text[svg_start..].find('>').map(|e| svg_start + e).unwrap_or(svg_text.len());
                let svg_tag = &svg_text[svg_start..tag_end];

                if let Some(vb) = Self::extract_attr(svg_tag, "viewBox") {
                    let nums: Vec<f32> = vb.split(|c: char| c.is_whitespace() || c == ',').filter_map(|s| s.parse::<f32>().ok()).collect();
                    if nums.len() >= 4 && nums[2] > 0.0 && nums[3] > 0.0 {
                        min_x = nums[0];
                        min_y = nums[1];
                        v_w = nums[2];
                        v_h = nums[3];
                        w = v_w;
                        h = v_h;
                    }
                }

                if let Some(w_str) = Self::extract_attr(svg_tag, "width")
                    && let Some(val) = Self::parse_dim(w_str)
                    && val > 0.0
                {
                    w = val;
                }
                if let Some(h_str) = Self::extract_attr(svg_tag, "height")
                    && let Some(val) = Self::parse_dim(h_str)
                    && val > 0.0
                {
                    h = val;
                }
            }

            let canvas_w = (w.round() as u32).clamp(1, 4096);
            let canvas_h = (h.round() as u32).clamp(1, 4096);
            (canvas_w, canvas_h, min_x, min_y, v_w, v_h)
        };

        let mut buffer = ImageBuffer::new(width, height)?;
        for y in 0..height {
            for x in 0..width {
                let _ = buffer.set_pixel(x, y, self.background);
            }
        }

        let scale_x = width as f32 / vb_w.max(1.0);
        let scale_y = height as f32 / vb_h.max(1.0);
        let off_x = -vb_min_x * scale_x;
        let off_y = -vb_min_y * scale_y;
        let transform = SvgTransform { scale_x, scale_y, off_x, off_y };

        // Iterate through XML tags
        let mut curr_group_fill: Option<crate::color::RgbColor> = None;
        let mut curr_group_stroke: Option<crate::color::RgbColor> = None;

        let mut idx = 0;
        let bytes = svg_text.as_bytes();

        while idx < bytes.len() {
            if bytes[idx] == b'<'
                && let Some(close_rel) = svg_text[idx..].find('>')
            {
                let tag = &svg_text[idx..idx + close_rel + 1];
                idx += close_rel + 1;

                if tag.starts_with("<g") {
                    if let Some(f) = Self::extract_attr(tag, "fill") {
                        curr_group_fill = Self::parse_color(f);
                    }
                    if let Some(s) = Self::extract_attr(tag, "stroke") {
                        curr_group_stroke = Self::parse_color(s);
                    }
                } else if tag.starts_with("</g>") {
                    curr_group_fill = None;
                    curr_group_stroke = None;
                } else if tag.starts_with("<rect") {
                    let fill =
                        Self::extract_attr(tag, "fill").and_then(Self::parse_color).or(curr_group_fill).unwrap_or(crate::color::RgbColor::BLACK);

                    let rx = Self::extract_attr(tag, "x").and_then(Self::parse_dim).unwrap_or(0.0);
                    let ry = Self::extract_attr(tag, "y").and_then(Self::parse_dim).unwrap_or(0.0);
                    let rw = Self::extract_attr(tag, "width").and_then(Self::parse_dim).unwrap_or(0.0);
                    let rh = Self::extract_attr(tag, "height").and_then(Self::parse_dim).unwrap_or(0.0);

                    let px_min = (rx * scale_x + off_x).max(0.0) as u32;
                    let py_min = (ry * scale_y + off_y).max(0.0) as u32;
                    let px_max = ((rx + rw) * scale_x + off_x).min(width as f32) as u32;
                    let py_max = ((ry + rh) * scale_y + off_y).min(height as f32) as u32;

                    for y in py_min..py_max {
                        for x in px_min..px_max {
                            let _ = buffer.set_pixel(x, y, fill);
                        }
                    }
                } else if tag.starts_with("<circle") {
                    let fill =
                        Self::extract_attr(tag, "fill").and_then(Self::parse_color).or(curr_group_fill).unwrap_or(crate::color::RgbColor::BLACK);

                    let cx = Self::extract_attr(tag, "cx").and_then(Self::parse_dim).unwrap_or(0.0);
                    let cy = Self::extract_attr(tag, "cy").and_then(Self::parse_dim).unwrap_or(0.0);
                    let r = Self::extract_attr(tag, "r").and_then(Self::parse_dim).unwrap_or(0.0);

                    let pcx = cx * scale_x + off_x;
                    let pcy = cy * scale_y + off_y;
                    let pr = r * ((scale_x + scale_y) * 0.5);

                    let min_x = (pcx - pr).max(0.0) as u32;
                    let max_x = (pcx + pr).min((width - 1) as f32) as u32;
                    let min_y = (pcy - pr).max(0.0) as u32;
                    let max_y = (pcy + pr).min((height - 1) as f32) as u32;

                    let r_sq = pr * pr;
                    for y in min_y..=max_y {
                        let dy = y as f32 - pcy;
                        for x in min_x..=max_x {
                            let dx = x as f32 - pcx;
                            if dx * dx + dy * dy <= r_sq {
                                let _ = buffer.set_pixel(x, y, fill);
                            }
                        }
                    }
                } else if tag.starts_with("<line") {
                    let stroke =
                        Self::extract_attr(tag, "stroke").and_then(Self::parse_color).or(curr_group_stroke).unwrap_or(crate::color::RgbColor::BLACK);

                    let x1 = Self::extract_attr(tag, "x1").and_then(Self::parse_dim).unwrap_or(0.0);
                    let y1 = Self::extract_attr(tag, "y1").and_then(Self::parse_dim).unwrap_or(0.0);
                    let x2 = Self::extract_attr(tag, "x2").and_then(Self::parse_dim).unwrap_or(0.0);
                    let y2 = Self::extract_attr(tag, "y2").and_then(Self::parse_dim).unwrap_or(0.0);

                    let px1 = (x1 * scale_x + off_x).round() as i32;
                    let py1 = (y1 * scale_y + off_y).round() as i32;
                    let px2 = (x2 * scale_x + off_x).round() as i32;
                    let py2 = (y2 * scale_y + off_y).round() as i32;

                    Self::draw_line(&mut buffer, px1, py1, px2, py2, stroke);
                } else if tag.starts_with("<path") {
                    let fill = Self::extract_attr(tag, "fill").and_then(Self::parse_color).or(curr_group_fill);

                    let stroke = Self::extract_attr(tag, "stroke").and_then(Self::parse_color).or(curr_group_stroke);

                    if let Some(d) = Self::extract_attr(tag, "d") {
                        Self::render_path_d(&mut buffer, d, transform, fill, stroke);
                    }
                }
                continue;
            }
            idx += 1;
        }

        Ok(buffer)
    }

    /// Bresenham's line algorithm.
    fn draw_line(buffer: &mut ImageBuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: crate::color::RgbColor) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let w = buffer.width as i32;
        let h = buffer.height as i32;

        loop {
            if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
                let _ = buffer.set_pixel(x0 as u32, y0 as u32, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Parse and render SVG path command string.
    fn render_path_d(
        buffer: &mut ImageBuffer,
        d: &str,
        transform: SvgTransform,
        fill: Option<crate::color::RgbColor>,
        stroke: Option<crate::color::RgbColor>,
    ) {
        let mut points: Vec<(f32, f32)> = Vec::new();
        let mut cur_x = 0.0_f32;
        let mut cur_y = 0.0_f32;
        let mut start_x = 0.0_f32;
        let mut start_y = 0.0_f32;

        let tokens: Vec<&str> = d.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()).collect();
        let mut t_idx = 0;

        while t_idx < tokens.len() {
            let tok = tokens[t_idx];
            let cmd = tok.chars().next().unwrap_or(' ');
            let rest = &tok[1..];

            // Helper to fetch next float token
            let mut get_next_num = || -> Option<f32> {
                if !rest.is_empty() && t_idx == 0 {
                    // Number was attached to command
                }
                t_idx += 1;
                if t_idx < tokens.len() { tokens[t_idx].parse::<f32>().ok() } else { None }
            };

            match cmd {
                'M' => {
                    if let (Some(x), Some(y)) = (get_next_num(), get_next_num()) {
                        cur_x = x;
                        cur_y = y;
                        start_x = cur_x;
                        start_y = cur_y;
                        points.push((cur_x, cur_y));
                    }
                }
                'm' => {
                    if let (Some(dx), Some(dy)) = (get_next_num(), get_next_num()) {
                        cur_x += dx;
                        cur_y += dy;
                        start_x = cur_x;
                        start_y = cur_y;
                        points.push((cur_x, cur_y));
                    }
                }
                'L' => {
                    if let (Some(x), Some(y)) = (get_next_num(), get_next_num()) {
                        cur_x = x;
                        cur_y = y;
                        points.push((cur_x, cur_y));
                    }
                }
                'l' => {
                    if let (Some(dx), Some(dy)) = (get_next_num(), get_next_num()) {
                        cur_x += dx;
                        cur_y += dy;
                        points.push((cur_x, cur_y));
                    }
                }
                'H' => {
                    if let Some(x) = get_next_num() {
                        cur_x = x;
                        points.push((cur_x, cur_y));
                    }
                }
                'h' => {
                    if let Some(dx) = get_next_num() {
                        cur_x += dx;
                        points.push((cur_x, cur_y));
                    }
                }
                'V' => {
                    if let Some(y) = get_next_num() {
                        cur_y = y;
                        points.push((cur_x, cur_y));
                    }
                }
                'v' => {
                    if let Some(dy) = get_next_num() {
                        cur_y += dy;
                        points.push((cur_x, cur_y));
                    }
                }
                'Z' | 'z' => {
                    cur_x = start_x;
                    cur_y = start_y;
                    points.push((cur_x, cur_y));
                    t_idx += 1;
                    continue;
                }
                _ => {
                    t_idx += 1;
                    continue;
                }
            }
        }

        if points.len() < 2 {
            return;
        }

        // Screen projected polygon points
        let screen_pts: Vec<(f32, f32)> =
            points.iter().map(|&(x, y)| (x * transform.scale_x + transform.off_x, y * transform.scale_y + transform.off_y)).collect();

        // Polygon scanline fill (even-odd rule)
        if let Some(fill_color) = fill {
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            for &(_, y) in &screen_pts {
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            }

            let start_y = (min_y.max(0.0).round() as i32).clamp(0, buffer.height as i32 - 1);
            let end_y = (max_y.min((buffer.height - 1) as f32).round() as i32).clamp(0, buffer.height as i32 - 1);

            for scan_y in start_y..=end_y {
                let y_f = scan_y as f32 + 0.5;
                let mut node_x = Vec::new();

                let n = screen_pts.len();
                for i in 0..n {
                    let j = (i + 1) % n;
                    let (x0, y0) = screen_pts[i];
                    let (x1, y1) = screen_pts[j];

                    if (y0 < y_f && y1 >= y_f) || (y1 < y_f && y0 >= y_f) {
                        let t = (y_f - y0) / (y1 - y0);
                        let x = x0 + t * (x1 - x0);
                        node_x.push(x);
                    }
                }

                node_x.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                for chunk in node_x.chunks(2) {
                    if chunk.len() == 2 {
                        let x_start = (chunk[0].max(0.0).round() as i32).clamp(0, buffer.width as i32 - 1) as u32;
                        let x_end = (chunk[1].min((buffer.width - 1) as f32).round() as i32).clamp(0, buffer.width as i32 - 1) as u32;
                        for px in x_start..=x_end {
                            let _ = buffer.set_pixel(px, scan_y as u32, fill_color);
                        }
                    }
                }
            }
        }

        // Stroke path edges if stroke color requested
        if let Some(stroke_color) = stroke {
            for i in 0..screen_pts.len() - 1 {
                let (x0, y0) = screen_pts[i];
                let (x1, y1) = screen_pts[i + 1];
                Self::draw_line(buffer, x0.round() as i32, y0.round() as i32, x1.round() as i32, y1.round() as i32, stroke_color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_rasterizer_rect_and_circle() {
        let svg = r##"<svg viewBox="0 0 100 100" width="100" height="100">
            <rect x="10" y="10" width="80" height="80" fill="#000000" />
            <circle cx="50" cy="50" r="20" fill="#FF0000" />
        </svg>"##;

        let rasterizer = SvgRasterizer::default();
        let buf = rasterizer.rasterize(svg).expect("rasterize should succeed");
        assert_eq!(buf.width, 100);
        assert_eq!(buf.height, 100);

        // Center pixel should be red circle
        let center_px = buf.get_pixel(50, 50).unwrap();
        assert_eq!(center_px, crate::color::RgbColor::new(255, 0, 0));

        // Corner pixel inside rect should be black
        let rect_px = buf.get_pixel(15, 15).unwrap();
        assert_eq!(rect_px, crate::color::RgbColor::BLACK);

        // Outside rect should be white background
        let bg_px = buf.get_pixel(2, 2).unwrap();
        assert_eq!(bg_px, crate::color::RgbColor::WHITE);
    }

    #[test]
    fn test_svg_rasterizer_path_polygon() {
        let svg = r##"<svg viewBox="0 0 50 50">
            <path d="M 10 10 L 40 10 L 40 40 L 10 40 Z" fill="#00FF00" />
        </svg>"##;

        let rasterizer = SvgRasterizer::default();
        let buf = rasterizer.rasterize(svg).expect("path rasterization should succeed");
        assert_eq!(buf.width, 50);
        assert_eq!(buf.height, 50);

        let inside = buf.get_pixel(25, 25).unwrap();
        assert_eq!(inside, crate::color::RgbColor::new(0, 255, 0));
    }

    #[test]
    fn test_svg_rasterizer_black_vector_roundtrip() {
        let mut original = ImageBuffer::new(30, 30).unwrap();
        for y in 0..30 {
            for x in 0..30 {
                let color = if x > 10 && x < 20 && y > 10 && y < 20 { crate::color::RgbColor::BLACK } else { crate::color::RgbColor::WHITE };
                let _ = original.set_pixel(x, y, color);
            }
        }

        let isolator = BlackVectorIsolation { max_luminance_threshold: 128, pixel_size: 1.0 };
        let svg = isolator.trace_to_svg(&original).expect("trace should succeed");

        let rasterizer = SvgRasterizer::default();
        let reconstructed = rasterizer.rasterize(&svg).expect("rasterize should succeed");
        assert_eq!(reconstructed.width, 30);
        assert_eq!(reconstructed.height, 30);

        let center_px = reconstructed.get_pixel(15, 15).unwrap();
        assert_eq!(center_px, crate::color::RgbColor::BLACK);

        let corner_px = reconstructed.get_pixel(5, 5).unwrap();
        assert_eq!(corner_px, crate::color::RgbColor::WHITE);
    }
}
