use serde::{Deserialize, Serialize};

/// 2D floating-point geometric point.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

impl Point2D {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    #[inline]
    pub fn perpendicular_distance_to_segment(&self, start: &Self, end: &Self) -> f32 {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let line_len_sq = dx * dx + dy * dy;

        if line_len_sq < 1e-9 {
            return self.distance_to(start);
        }

        let cross = ((end.y - start.y) * self.x - (end.x - start.x) * self.y + end.x * start.y - end.y * start.x).abs();
        cross / line_len_sq.sqrt()
    }
}

/// Ramer-Douglas-Peucker (RDP) polyline simplification algorithm.
/// Reduces number of vertices while preserving overall curve morphology within epsilon tolerance.
pub fn rdp_simplify(points: &[Point2D], epsilon: f32) -> Vec<Point2D> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut max_dist = 0.0f32;
    let mut index = 0;
    let start = points[0];
    let end = points[points.len() - 1];

    for (i, p) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let dist = p.perpendicular_distance_to_segment(&start, &end);
        if dist > max_dist {
            max_dist = dist;
            index = i;
        }
    }

    if max_dist > epsilon {
        let left = rdp_simplify(&points[..=index], epsilon);
        let right = rdp_simplify(&points[index..], epsilon);

        let mut result = left;
        result.pop(); // remove duplicate middle point
        result.extend(right);
        result
    } else {
        vec![start, end]
    }
}
