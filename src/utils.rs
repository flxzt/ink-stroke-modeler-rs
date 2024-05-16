// utilities
use std::ops::{Add, Mul, Sub};

// clamp : use clamp(self,min,max) with 0 and 1 for floats

/// interpol value
/// get the value to use for the interpolation
/// 0 if value < start
/// 1 if value > end
/// and (value - start)/(end - start) otherwise
pub fn normalize01_32(start: f32, end: f32, value: f32) -> f32 {
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

pub fn interp<T>(start: T, end: T, interp_amount: f32) -> T
where
    T: Sub<Output = T>,
    T: Add<Output = T>,
    T: Copy,
    T: Mul<f32, Output = T>,
{
    return start + (end - start) * interp_amount.clamp(0.0, 1.0);
}

pub fn interp2(start: (f32, f32), end: (f32, f32), interp_amount: f32) -> (f32, f32) {
    return (
        start.0 + interp_amount.clamp(0.0, 1.0) * (end.0 - start.0),
        start.1 + interp_amount.clamp(0.0, 1.0) * (end.1 - start.1),
    );
}

// returns the point on the line segment from `segment_start` to `segment_end`
// that is closest to `point`, represented as the ratio of the length
// along the segment
pub fn nearest_point_on_segment(start: (f32, f32), end: (f32, f32), point: (f32, f32)) -> f32 {
    if start == end {
        0.0 as f32
    } else {
        let seg_vector = (end.0 - start.0, end.1 - start.1);
        let proj_vector = (point.0 - start.0, point.1 - start.1);

        return (dot(proj_vector, seg_vector) / dot(seg_vector, seg_vector)).clamp(0.0, 1.0);
    }
}

pub fn dot(x: (f32, f32), y: (f32, f32)) -> f32 {
    x.0 * y.0 + x.1 * y.1
}
