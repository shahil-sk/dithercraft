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
