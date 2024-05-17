use std::collections::VecDeque;

// modules
pub mod engine;
pub mod impl_ds;
pub mod position_modeler;
pub mod state_modeler;
pub mod testing;
pub mod utils;

use position_modeler::PositionModeler;
// imports
use engine::WobbleSample;
use state_modeler::StateModeler;

// this file contains all structs that are used in the program
/// Kalman predictor data
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct KalmanPredictorParams {
    /// the variance of the noise inherent to the stroke itself
    ///
    /// Should be >0
    pub process_noise: f64,
    /// the variance of the noise that rises from errors in measurements
    /// of the stroke
    ///
    /// Should be >0
    pub measurement_noise: f64,
    /// the minimum number of inputs received before the kalman predictor
    /// is considered stable enough to make a prediction
    ///
    /// Should be >0
    pub min_stable_iteration: i32,
    /// the kalman filter assumes that input is received in uniform time
    /// steps but this is not always the case (...)
    /// We hold onto the most recent timestamps for use in calculating
    /// the correction for this. This determines the maximum number of
    /// timestamps to save
    ///
    /// Should be > 0
    pub max_time_samples: i32,
    /// the minimum allowed velocity of the "catch up" portion of the
    /// prediction, which covers the distance between the last result (
    /// the last corrected position) and the latest input
    ///
    /// A good starting point is 3 order of magnitude smaller than the
    /// expected speed of the inputs
    ///
    /// Should be >0
    pub min_catchup_velocity: f32,
    /// These weights are applied to the acceleration (x^2) and jerk (x^3)
    /// of the cubic prediction polynomial. The closer they are to zero,
    /// the more linear the prediction will be
    ///
    /// (weight for acceleration)
    ///
    /// Should be finite
    pub acceleration_weight: f32,
    /// (weight for jerk)
    ///
    /// Should be finite
    pub jerk_weight: f32,
    /// this value is a hint to the predictor, indicating the desired duration
    /// of the portion of the prediction extending beyond the position of the
    /// last input. The actual duration of that portion of the prediction may
    /// be less than this, based on the predictor's confidence, but it will
    /// never be greater
    ///
    /// DURATION
    ///
    /// should be >0
    pub prediction_interval: f64,
    /// The Kalman predictor uses several heuristics to evaluate confidence
    /// in the prediction. Each heuristic produces a confidence value between
    /// 0 and 1, and then we take their product as the total confidence.
    /// These parameters may be used to tune those heuristics
    ///
    /// The first heuristic simply increases confidence as we receive more
    /// samples (i.e. input points). It evaluates to 0 at no samples,
    /// and 1 at [KalmanPredictorParams::confidence_desired_number_of_samples]
    ///
    /// should be >0
    pub confidence_desired_number_of_samples: i32,
    /// The second heuristic is based on the distance between the last sample
    /// and the current estimate (which one ?). If the distance is 0, it
    /// evaluates to 1, and if the distance is greater than or equal to
    /// [KalmanPredictorParams::confidence_max_estimation_distance], it
    /// evaluates to 0. (linear interpolation between the values ?)
    ///
    /// A good starting point is 1.5 times
    /// [KalmanPredictorParams::measurement_noise]
    ///
    /// Should be > 0
    pub confidence_max_estimation_distance: f32,
    /// The third heuristic is based on the speed of the prediction,
    /// which is approximated by measuring from the start of the prediction
    /// to the projected endpoint (if it were extended to the full
    /// [KalmanPredictorParams::prediction_interval])
    /// It evaluates to 0 at [KalmanPredictorParams::confidence_min_travel_speed]
    /// and 1 at [KalmanPredictorParams::confidence_max_travel_speed]
    ///
    /// Good starting points are 5% and 25% of the expected speed of the inputs
    ///
    /// Linear interpolation between these values ?
    ///
    /// Should be >0 and finite, with the max value >= min value
    pub confidence_min_travel_speed: f32,
    pub confidence_max_travel_speed: f32,
    /// The fourth heuristic is based on the linearity of the prediction,
    /// which is approximated by comparing the endpoint of the prediction with
    /// the endpoint of a linear prediction (again, extending to the full
    /// [KalmanPredictorParams::prediction_interval]).
    ///
    /// It evaluates to 1 at zero distance and
    /// [KalmanPredictorParams::confidence_baseline_linearity_confidence]
    /// at a distance of
    /// [KalmanPredictorParams::confidence_max_linear_deviation]
    ///
    /// Linear interpolation between the two ?
    ///
    /// should be >0
    pub confidence_max_linear_deviation: f32,
    /// should be in the interval \[0,1\]
    pub confidence_baseline_linearity_confidence: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelerInput {
    event_type: ModelerInputEventType,
    pos: (f32, f32),
    time: f64,
    pressure: f32,
    // tilt and orientation are optional parameters, so we remove them here to
    // make our lives easier
}

impl Default for ModelerInput {
    fn default() -> Self {
        Self {
            event_type: ModelerInputEventType::kDown,
            pos: (0.0, 0.0),
            time: 0.0,
            pressure: 0.0,
        }
    }
}

// modeler Input event Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
pub enum ModelerInputEventType {
    kDown,
    kMove,
    kUp,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ModelerParams {
    /// these parameters are used to apply smoothing to the input to reduce
    /// wobble in the prediction
    ///
    /// The length of the window over which the moving average of speed and position is calculated
    ///
    /// Check if this can't be done with the rust time types as this probably comes from a
    /// conversion to float (DURATION)
    ///
    /// A good starting point is
    ///    <math>
    ///    <mrow>
    ///      <mn>2.5</mn>
    ///      <mi>/</mi>
    ///      <msub>
    ///        <mi>f</mi>
    ///        <mtext>input rate</mtext>
    ///      </msub>
    ///    </mrow>
    ///  </math>
    /// Should be positive
    pub wobble_smoother_timeout: f64,
    /// The range of speed considered for wobble smoothing.
    /// At [ModelerParams::wobble_smoother_speed_floor] the maximum
    /// amount of smoothing is applied. At [ModelerParams::wobble_smoother_speed_ceiling],
    /// no smoothing is applied
    ///
    /// Good starting points are 2 - 3 % of the expected speed of the inputs
    /// Should be positive and the speed floor smaller than the ceiling
    pub wobble_smoother_speed_floor: f32,
    pub wobble_smoother_speed_ceiling: f32,
    /// The mass of the "weight" being pulled along the path, multiplied by the spring constant.
    ///
    /// Should be positive
    pub position_modeler_spring_mass_constant: f32,
    /// The ratio of the pen's velocity that is subtracted from the pen's acceleration per unit time, to simulate drag.
    ///
    /// Should be positive
    pub position_modeler_drag_constant: f32,
    /// The minimum number of modeled inputs to output per unit time. If inputs are received at a lower rate,
    /// they will be upsampled to produce output of atleast [ModelerParams::sampling_min_output_rate].
    /// If inputs are received at a higher rate, the output rate will match the input rate.
    ///
    /// Should be positive
    pub sampling_min_output_rate: f64,
    /// This determines the stop condition for the end-of-stroke modeling
    /// If the position is within this distance of the final raw input, or
    /// if the last update iteration moved less than this distance,
    /// it stops iterating.
    ///
    /// this should be a small distance, good heuristic is
    /// 2-3 orders of magnitude smaller than the expected distance
    /// between input points
    ///
    /// Should be positive
    pub sampling_end_of_stroke_stopping_distance: f32,
    /// The maximum number of iterations to perform at the end of the stroke,
    /// if it does not stop due to the constraint of the `sampling_end_of_stroke_stopping_distance`
    ///
    /// Should be positive and is capped at 1000 (to limit the memory requirements)
    pub sampling_end_of_stroke_max_iterations: usize,
    /// Maximum number of outputs to generate per call to Update or Predict.
    /// related to issues if input events are received with too long of a delay
    /// See what's done in the rnote call and on this end to limit things like this
    ///
    /// Should be strictly positive
    pub sampling_max_outputs_per_call: usize,
    /// the maximum number of raw inputs to look at when
    /// searching for the nearest states when interpolating
    ///
    /// Should be strictly positive
    pub stylus_state_modeler_max_input_samples: usize,
}

// result struct
#[derive(Debug, PartialEq)]
pub struct ModelerResult {
    pos: (f32, f32),
    velocity: (f32, f32),
    acceleration: (f32, f32),
    time: f64,
    pressure: f32,
}

/// same as before but no pressure
///
#[derive(Copy, Clone, Debug)]
pub struct ModelerPartial {
    pos: (f32, f32),
    velocity: (f32, f32),
    acceleration: (f32, f32),
    time: f64,
}

/// This class models a stroke from a raw input stream. The modeling is performed in
/// several stages
/// - Wobble smoothing : dampens high-frequency noise from quantization error
/// - Position modeling : models the pen tip as a mass, connected by a spring, to a moving
/// anchor
/// - Stylus state modeling : constructs stylus states for modeled positions by interpolating
/// over the raw input
///
/// Additional, this class provides prediction of the modeled stroke
///
/// StrokeModeler is unit-agnostic
pub struct StrokeModeler {
    // all configuration parameters
    params: ModelerParams,
    /// wobble smoother structures
    /// deque to hold events that are recent
    /// to calculate a moving average
    wobble_decque: VecDeque<WobbleSample>,
    /// running weighted sum
    wobble_weighted_pos_sum: (f32, f32),
    /// running duration sum
    wobble_duration_sum: f64,
    /// running distance sum
    wobble_distance_sum: f32,
    // physical model for the stroke
    // only created on the first stroke
    position_modeler: Option<PositionModeler>,
    last_event: Option<ModelerInput>,
    state_modeler: StateModeler,
}
