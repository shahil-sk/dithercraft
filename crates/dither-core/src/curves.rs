//! Interactive Tone Curves & Monotonic Spline Interpolation.
//!
//! Provides parametric tone curve editing with monotonic cubic Hermite interpolation
//! (Fritsch-Carlson method) ensuring no spurious overshoots or non-monotonic oscillations.
//! Evaluates into 256-element look-up tables (LUT) for zero-allocation $O(N)$ parallel pixel remapping.

use crate::buffer::ImageBuffer;
use crate::error::DitherError;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// A normalized control point on a 2D tone curve ($x, y \in [0.0, 1.0]$).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f32,
    pub y: f32,
}

impl CurvePoint {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x: x.clamp(0.0, 1.0), y: y.clamp(0.0, 1.0) }
    }
}

/// A monotonic spline tone curve.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToneCurve {
    pub points: Vec<CurvePoint>,
}

impl Default for ToneCurve {
    fn default() -> Self {
        Self::linear()
    }
}

impl ToneCurve {
    /// Create a neutral linear identity tone curve (`y = x`).
    pub fn linear() -> Self {
        Self { points: vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)] }
    }

    /// Create an S-curve preset increasing contrast.
    pub fn s_curve() -> Self {
        Self { points: vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(0.25, 0.15), CurvePoint::new(0.75, 0.85), CurvePoint::new(1.0, 1.0)] }
    }

    /// Create an inverted negative tone curve (`y = 1 - x`).
    pub fn invert() -> Self {
        Self { points: vec![CurvePoint::new(0.0, 1.0), CurvePoint::new(1.0, 0.0)] }
    }

    /// Create a high-contrast solarize tone curve.
    pub fn solarize() -> Self {
        Self { points: vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(0.5, 1.0), CurvePoint::new(1.0, 0.0)] }
    }

    /// Insert or update a point on the curve, maintaining sorted order by X.
    pub fn set_point(&mut self, x: f32, y: f32) {
        let x = x.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);
        if let Some(existing) = self.points.iter_mut().find(|p| (p.x - x).abs() < 0.015) {
            existing.y = y;
        } else {
            self.points.push(CurvePoint::new(x, y));
            self.points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        }
    }

    /// Remove a point by index (first and last points cannot be deleted).
    pub fn remove_point(&mut self, index: usize) -> bool {
        if index > 0 && index < self.points.len() - 1 && self.points.len() > 2 {
            self.points.remove(index);
            true
        } else {
            false
        }
    }

    /// Reset curve to identity linear state.
    pub fn reset_linear(&mut self) {
        self.points = vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)];
    }

    /// Check if the curve is functionally identity linear.
    pub fn is_identity(&self) -> bool {
        if self.points.len() == 2
            && (self.points[0].x - 0.0).abs() < 1e-4
            && (self.points[0].y - 0.0).abs() < 1e-4
            && (self.points[1].x - 1.0).abs() < 1e-4
            && (self.points[1].y - 1.0).abs() < 1e-4
        {
            return true;
        }
        false
    }

    /// Evaluate the tone curve into a 256-element lookup table using
    /// piecewise cubic Hermite monotonic interpolation (Fritsch-Carlson).
    pub fn evaluate_lut(&self) -> [u8; 256] {
        let mut lut = [0u8; 256];
        if self.points.is_empty() {
            for (i, slot) in lut.iter_mut().enumerate() {
                *slot = i as u8;
            }
            return lut;
        }

        if self.points.len() == 1 {
            let val = (self.points[0].y * 255.0).round().clamp(0.0, 255.0) as u8;
            lut.fill(val);
            return lut;
        }

        // Monotonic cubic interpolation
        let n = self.points.len();
        let mut h = vec![0.0f32; n - 1];
        let mut delta = vec![0.0f32; n - 1];

        for i in 0..n - 1 {
            h[i] = (self.points[i + 1].x - self.points[i].x).max(1e-6);
            delta[i] = (self.points[i + 1].y - self.points[i].y) / h[i];
        }

        let mut m = vec![0.0f32; n];
        m[0] = delta[0];
        m[n - 1] = delta[n - 2];
        for i in 1..n - 1 {
            m[i] = (delta[i - 1] + delta[i]) * 0.5;
        }

        // Fritsch-Carlson slope limiter for monotonicity
        for i in 0..n - 1 {
            if delta[i].abs() < 1e-6 {
                m[i] = 0.0;
                m[i + 1] = 0.0;
            } else {
                let alpha = m[i] / delta[i];
                let beta = m[i + 1] / delta[i];
                let dist = alpha * alpha + beta * beta;
                if dist > 9.0 {
                    let tau = 3.0 / dist.sqrt();
                    m[i] = tau * alpha * delta[i];
                    m[i + 1] = tau * beta * delta[i];
                }
            }
        }

        for (in_val, out_val) in lut.iter_mut().enumerate() {
            let x = (in_val as f32) / 255.0;

            if x <= self.points[0].x {
                *out_val = (self.points[0].y * 255.0).round().clamp(0.0, 255.0) as u8;
                continue;
            }
            if x >= self.points[n - 1].x {
                *out_val = (self.points[n - 1].y * 255.0).round().clamp(0.0, 255.0) as u8;
                continue;
            }

            // Find segment
            let mut seg = 0;
            for i in 0..n - 1 {
                if x >= self.points[i].x && x <= self.points[i + 1].x {
                    seg = i;
                    break;
                }
            }

            let t = ((x - self.points[seg].x) / h[seg]).clamp(0.0, 1.0);
            let t2 = t * t;
            let t3 = t2 * t;

            // Hermite basis functions
            let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
            let h10 = t3 - 2.0 * t2 + t;
            let h01 = -2.0 * t3 + 3.0 * t2;
            let h11 = t3 - t2;

            let y = h00 * self.points[seg].y + h10 * h[seg] * m[seg] + h01 * self.points[seg + 1].y + h11 * h[seg] * m[seg + 1];

            *out_val = (y * 255.0).round().clamp(0.0, 255.0) as u8;
        }

        lut
    }

    /// Apply this tone curve across all channels of an ImageBuffer in parallel.
    pub fn apply(&self, buffer: &mut ImageBuffer) {
        if self.is_identity() {
            return;
        }
        let lut = self.evaluate_lut();
        buffer.data.par_chunks_exact_mut(3).for_each(|pixel| {
            pixel[0] = lut[pixel[0] as usize];
            pixel[1] = lut[pixel[1] as usize];
            pixel[2] = lut[pixel[2] as usize];
        });
    }

    /// Apply this tone curve to a single channel (0=R, 1=G, 2=B) of an ImageBuffer.
    pub fn apply_channel(&self, buffer: &mut ImageBuffer, channel: usize) -> Result<(), DitherError> {
        if channel > 2 {
            return Err(DitherError::InvalidParameter("Channel must be 0 (R), 1 (G), or 2 (B)".into()));
        }
        if self.is_identity() {
            return Ok(());
        }
        let lut = self.evaluate_lut();
        buffer.data.par_chunks_exact_mut(3).for_each(|pixel| {
            pixel[channel] = lut[pixel[channel] as usize];
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tone_curve_linear_and_identity() {
        let curve = ToneCurve::linear();
        assert!(curve.is_identity());
        let lut = curve.evaluate_lut();
        for (i, &val) in lut.iter().enumerate() {
            assert_eq!(i as u8, val);
        }
    }

    #[test]
    fn test_tone_curve_invert() {
        let curve = ToneCurve::invert();
        assert!(!curve.is_identity());
        let lut = curve.evaluate_lut();
        assert_eq!(lut[0], 255);
        assert_eq!(lut[255], 0);
        assert_eq!(lut[128], 127);
    }

    #[test]
    fn test_tone_curve_s_curve_and_apply() {
        let curve = ToneCurve::s_curve();
        let lut = curve.evaluate_lut();
        assert_eq!(lut[0], 0);
        assert_eq!(lut[255], 255);
        // Midtone inflection
        assert!(lut[64] < 64);
        assert!(lut[192] > 192);

        let mut img = ImageBuffer::new(4, 4).expect("create buffer");
        img.data.fill(64);
        curve.apply(&mut img);
        assert_eq!(img.data[0], lut[64]);
    }

    #[test]
    fn test_tone_curve_channel_isolation() {
        let curve = ToneCurve::invert();
        let mut img = ImageBuffer::new(2, 2).expect("create buffer");
        img.data.fill(0);
        curve.apply_channel(&mut img, 0).expect("apply red");
        assert_eq!(img.data[0], 255); // Red inverted
        assert_eq!(img.data[1], 0); // Green unchanged
        assert_eq!(img.data[2], 0); // Blue unchanged
    }
}
