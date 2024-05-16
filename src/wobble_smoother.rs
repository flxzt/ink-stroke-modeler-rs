// datastructures for the wobble smoother and logic

// smooth out the input position from high frequency noise
// uses a moving average of position and interpolating between this
// position and the raw position based on the speed.
// high speeds movements won't be smoothed but low speed will.

/// wrapper time to include all needed information
/// in the Deque
#[derive(Debug)]
pub struct WobbleSample {
    /// raw position
    pub position: (f32, f32),
    /// position weighted by the duration
    pub weighted_position: (f32, f32),
    /// distance to the previous element
    pub distance: f32,
    /// time distance to the previous element
    pub duration: f64,
    /// time of the event
    pub time: f64,
}
