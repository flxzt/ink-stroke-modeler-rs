// utilities
use std::ops::{Add, Mul, Sub};

// clamp : use clamp(self,min,max) with 0 and 1 for floats

/// interpol value
/// get the value to use for the interpolation
/// 0 if value < start
/// 1 if value > end
/// and (value - start)/(end - start) otherwise
pub(crate) fn normalize01_32(start: f32, end: f32, value: f32) -> f32 {
    if start == end {
        if value > start {
            1.0
        } else {
            0.0
        }
    } else {
        ((value - start) / (end - start)).clamp(0.0, 1.0)
    }
}

/// interpolate the value
///
/// normal interpolation clamped to \[0,1\] for the `interp_amount`
pub(crate) fn interp<T>(start: T, end: T, interp_amount: f32) -> T
where
    T: Sub<Output = T>,
    T: Add<Output = T>,
    T: Copy,
    T: Mul<f32, Output = T>,
{
    start + (end - start) * interp_amount.clamp(0.0, 1.0)
}

/// interpolation (with the `interp_amount` clamped between 0 and 1) for `(f32,f32)` types
pub(crate) fn interp2(start: (f32, f32), end: (f32, f32), interp_amount: f32) -> (f32, f32) {
    (
        start.0 + interp_amount.clamp(0.0, 1.0) * (end.0 - start.0),
        start.1 + interp_amount.clamp(0.0, 1.0) * (end.1 - start.1),
    )
}

/// returns the point on the line segment from `segment_start` to `segment_end`
/// that is closest to `point`, represented as the ratio of the length
/// along the segment
pub(crate) fn nearest_point_on_segment(start: (f32, f32), end: (f32, f32), point: (f32, f32)) -> f32 {
    if start == end {
        0.0_f32
    } else {
        let seg_vector = (end.0 - start.0, end.1 - start.1);
        let proj_vector = (point.0 - start.0, point.1 - start.1);

        (dot(proj_vector, seg_vector) / dot(seg_vector, seg_vector)).clamp(0.0, 1.0)
    }
}

/// dot product for `(f32,32)` types
pub(crate) fn dot(x: (f32, f32), y: (f32, f32)) -> f32 {
    x.0 * y.0 + x.1 * y.1
}

/// distance calculation for `(f32,f32)` types
pub fn dist(start: (f32, f32), end: (f32, f32)) -> f32 {
    ((start.0 - end.0).powi(2) + (start.1 - end.1).powi(2)).sqrt()
}