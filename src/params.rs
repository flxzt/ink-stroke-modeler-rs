/// all parameters for the modeler
#[derive(Debug, Clone, PartialEq, PartialOrd, Copy)]
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
