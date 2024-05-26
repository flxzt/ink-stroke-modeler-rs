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

impl ModelerParams {
    /// [ModelerParams::wobble_smoother_timeout] : 0.04,\
    /// [ModelerParams::wobble_smoother_speed_floor] : 1.31,\
    /// [ModelerParams::wobble_smoother_speed_ceiling] : 1.44,\
    /// [ModelerParams::position_modeler_spring_mass_constant] : 11.0 / 32400.0,\
    /// [ModelerParams::position_modeler_drag_constant] : 72.0,\
    /// [ModelerParams::sampling_min_output_rate] : 180.0,\
    /// [ModelerParams::sampling_end_of_stroke_stopping_distance] : 0.001,\
    /// [ModelerParams::sampling_end_of_stroke_max_iterations] : 20,\
    /// [ModelerParams::sampling_max_outputs_per_call] : 20,\
    /// [ModelerParams::stylus_state_modeler_max_input_samples] : 10,
    pub fn suggested() -> Self {
        Self {
            wobble_smoother_timeout: 0.04,
            wobble_smoother_speed_floor: 1.31,
            wobble_smoother_speed_ceiling: 1.44,
            position_modeler_spring_mass_constant: 11.0 / 32400.0,
            position_modeler_drag_constant: 72.0,
            sampling_min_output_rate: 180.0,
            sampling_end_of_stroke_stopping_distance: 0.001,
            sampling_end_of_stroke_max_iterations: 20,
            sampling_max_outputs_per_call: 20,
            stylus_state_modeler_max_input_samples: 10,
        }
    }

    /// validate the parameters as being correct, returns a error string with
    /// the reasons otherwise
    pub fn validate(self) -> Result<Self, String> {
        let parameter_tests = [
            self.position_modeler_spring_mass_constant > 0.0,
            self.position_modeler_drag_constant > 0.0,
            self.sampling_min_output_rate > 0.0,
            self.sampling_end_of_stroke_stopping_distance > 0.0,
            self.sampling_end_of_stroke_max_iterations > 0,
            self.sampling_end_of_stroke_max_iterations < 1000,
            self.sampling_max_outputs_per_call > 0,
            self.wobble_smoother_timeout > 0.0,
            self.wobble_smoother_speed_floor > 0.0,
            self.wobble_smoother_speed_ceiling > 0.0,
            self.wobble_smoother_speed_floor < self.wobble_smoother_speed_ceiling,
        ];

        let errors = vec![
            "`position_modeler_spring_mass_constant` is not positive; ",
            "`position_modeler_drag_constant` is not positive; ",
            "`sampling_min_output_rate` is not positive; ",
            "`sampling_end_of_stroke_stopping_distance` is not positive; ",
            "`sampling_end_of_stroke_max_iterations` is not positive; ",
            "`sampling_end_of_stroke_max_iterations` is too large (>1000); ",
            "`sampling_max_outputs_per_call` is not positive; ",
            "`wobble_smoother_timeout` is not positive; ",
            "`wobble_smoother_speed_floor` is not positive; ",
            "`wobble_smoother_speed_ceiling` is not positive; ",
            "`wobble_smoother_speed_floor` should be strictly smaller than `wobble_smoother_speed_ceiling`",
        ];

        let tests_passed = parameter_tests.iter().fold(true, |acc, x| acc & x);

        if tests_passed {
            Ok(self)
        } else {
            //Collect errors
            let error_acc = parameter_tests
                .iter()
                .zip(errors)
                .filter(|x| !*(x.0))
                .fold(String::from("the following errors occured : "), |acc, x| {
                    acc + x.1
                });

            Err(error_acc)
        }
    }
}

#[cfg(test)]
mod test_params {
    // import parent
    use super::super::*;
    #[test]
    fn validation_modeler_params() {
        let s = (ModelerParams {
            wobble_smoother_timeout: -1.0,
            wobble_smoother_speed_floor: -1.0,
            wobble_smoother_speed_ceiling: -1.0,
            position_modeler_spring_mass_constant: -1.0,
            position_modeler_drag_constant: -1.0,
            sampling_min_output_rate: -1.0,
            sampling_end_of_stroke_stopping_distance: -1.0,
            sampling_end_of_stroke_max_iterations: 0,
            sampling_max_outputs_per_call: 0,
            stylus_state_modeler_max_input_samples: 0,
        })
        .validate();
        match s {
            Ok(_) => assert!(false),
            Err(_) => assert!(true),
        }
    }
}
