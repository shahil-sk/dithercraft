//! Piecewise Cubic Bézier Curve Fitting for Vector Plotters and SVG Exporters.
//!
//! Fits C1-continuous cubic Bézier curves over polyline vertices, converting
//! discrete pixel steps into smooth, organic strokes for pen plotters and screen printing.

use super::rdp::Point2D;

/// A cubic Bézier curve segment defined by start, two control points, and end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezierSegment {
    pub p0: Point2D,
    pub c1: Point2D,
    pub c2: Point2D,
    pub p1: Point2D,
}

impl CubicBezierSegment {
    pub fn new(p0: Point2D, c1: Point2D, c2: Point2D, p1: Point2D) -> Self {
        Self { p0, c1, c2, p1 }
    }

    /// Format this segment as an SVG cubic curve command: `C c1x c1y, c2x c2y, p1x p1y`
    pub fn to_svg_command(&self) -> String {
        format!("C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}", self.c1.x, self.c1.y, self.c2.x, self.c2.y, self.p1.x, self.p1.y)
    }
}

/// Fit a smooth piecewise cubic Bézier spline through an ordered sequence of polyline points.
///
/// Uses standard Catmull-Rom tangent projection to compute C1-continuous control points.
pub fn fit_cubic_spline(points: &[Point2D], tension: f32) -> Vec<CubicBezierSegment> {
    if points.len() < 2 {
        return Vec::new();
    }

    let alpha = (tension / 3.0).clamp(0.0, 1.0);
    let mut segments = Vec::with_capacity(points.len() - 1);

    for i in 0..points.len() - 1 {
        let p0 = points[i];
        let p1 = points[i + 1];

        let p_prev = if i == 0 { p0 } else { points[i - 1] };
        let p_next = if i + 2 < points.len() { points[i + 2] } else { p1 };

        let c1 = Point2D::new(p0.x + alpha * (p1.x - p_prev.x), p0.y + alpha * (p1.y - p_prev.y));
        let c2 = Point2D::new(p1.x - alpha * (p_next.x - p0.x), p1.y - alpha * (p_next.y - p0.y));

        segments.push(CubicBezierSegment::new(p0, c1, c2, p1));
    }

    segments
}

/// Convert a list of cubic Bézier segments into a complete SVG `<path d="M... C...">` string.
pub fn spline_to_svg_path(segments: &[CubicBezierSegment], stroke_color: &str, stroke_width: f32) -> String {
    if segments.is_empty() {
        return String::new();
    }

    let start = segments[0].p0;
    let mut d = format!("M {:.2} {:.2}", start.x, start.y);

    for seg in segments {
        d.push(' ');
        d.push_str(&seg.to_svg_command());
    }

    format!(r#"<path d="{d}" fill="none" stroke="{stroke_color}" stroke-width="{stroke_width}" stroke-linecap="round" stroke-linejoin="round" />"#)
}

/// Trace an image buffer's dark pixels and export as a smooth piecewise cubic Bézier SVG.
pub fn export_smooth_bezier_svg(input: &crate::buffer::ImageBuffer, threshold: u8, tension: f32) -> Result<String, crate::error::DitherError> {
    let width = input.width;
    let height = input.height;

    let mut paths = Vec::new();
    for y in 0..height {
        let mut span_start = None;
        for x in 0..width {
            let px = input.get_pixel(x, y).unwrap_or(crate::color::RgbColor::WHITE);
            let is_dark = px.luminance() <= threshold;
            if is_dark {
                if span_start.is_none() {
                    span_start = Some(x);
                }
            } else if let Some(start_x) = span_start {
                let p1 = Point2D::new(start_x as f32, y as f32);
                let p2 = Point2D::new(x as f32, y as f32);
                let spline = fit_cubic_spline(&[p1, p2], tension);
                if !spline.is_empty() {
                    paths.push(spline_to_svg_path(&spline, "#000000", 1.0));
                }
                span_start = None;
            }
        }
        if let Some(start_x) = span_start {
            let p1 = Point2D::new(start_x as f32, y as f32);
            let p2 = Point2D::new(width as f32, y as f32);
            let spline = fit_cubic_spline(&[p1, p2], tension);
            if !spline.is_empty() {
                paths.push(spline_to_svg_path(&spline, "#000000", 1.0));
            }
        }
    }

    let mut svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">"#);
    svg.push('\n');
    for p in paths {
        svg.push_str("  ");
        svg.push_str(&p);
        svg.push('\n');
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubic_spline_fitting_and_svg_export() {
        let points = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 20.0), Point2D::new(30.0, 20.0), Point2D::new(40.0, 0.0)];

        let spline = fit_cubic_spline(&points, 0.5);
        assert_eq!(spline.len(), 3);
        assert_eq!(spline[0].p0, Point2D::new(0.0, 0.0));
        assert_eq!(spline[2].p1, Point2D::new(40.0, 0.0));

        let svg = spline_to_svg_path(&spline, "#00FF00", 1.5);
        assert!(svg.contains(r#"fill="none""#));
        assert!(svg.contains(r##"stroke="#00FF00""##));
        assert!(svg.contains("M 0.00 0.00"));
        assert!(svg.contains("C "));
    }
}
