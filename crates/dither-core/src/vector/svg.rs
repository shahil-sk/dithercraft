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
