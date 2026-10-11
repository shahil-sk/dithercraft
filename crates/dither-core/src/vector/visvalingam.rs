//! Visvalingam-Whyatt Area-Based Polyline Simplification.
//!
//! Progressively removes vertices with the smallest effective triangular area,
//! preserving organic curves and natural contours without the harsh angular artifacts
//! of perpendicular-distance methods.

use super::rdp::Point2D;

/// Calculate the unsigned triangular area formed by three points: (p0, p1, p2).
#[inline]
pub fn triangle_area(p0: Point2D, p1: Point2D, p2: Point2D) -> f32 {
    0.5 * (p0.x * (p1.y - p2.y) + p1.x * (p2.y - p0.y) + p2.x * (p0.y - p1.y)).abs()
}

/// Simplify a polyline using the Visvalingam-Whyatt area-elimination algorithm.
///
/// Progressively eliminates interior vertices whose effective triangular area
/// is less than `min_area_threshold`. The endpoints are strictly preserved.
pub fn visvalingam_simplify(points: &[Point2D], min_area_threshold: f32) -> Vec<Point2D> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut pts = points.to_vec();

    loop {
        if pts.len() <= 2 {
            break;
        }

        let mut min_idx = 1;
        let mut min_area = f32::MAX;

        for i in 1..pts.len() - 1 {
            let area = triangle_area(pts[i - 1], pts[i], pts[i + 1]);
            if area < min_area {
                min_area = area;
                min_idx = i;
            }
        }

        if min_area >= min_area_threshold {
            break;
        }

        pts.remove(min_idx);
    }

    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visvalingam_simplification() {
        // Collinear points have zero area and should be removed
        let pts = vec![Point2D::new(0.0, 0.0), Point2D::new(1.0, 1.0), Point2D::new(2.0, 2.0), Point2D::new(3.0, 3.0), Point2D::new(4.0, 4.0)];

        let simplified = visvalingam_simplify(&pts, 0.01);
        assert_eq!(simplified.len(), 2);
        assert_eq!(simplified[0], Point2D::new(0.0, 0.0));
        assert_eq!(simplified[1], Point2D::new(4.0, 4.0));

        // Significant corner with large area should be preserved
        let corner = vec![Point2D::new(0.0, 0.0), Point2D::new(2.0, 10.0), Point2D::new(4.0, 0.0)];
        let preserved = visvalingam_simplify(&corner, 5.0);
        assert_eq!(preserved.len(), 3);
    }
}
