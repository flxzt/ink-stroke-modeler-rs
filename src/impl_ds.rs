// import
use super::*;

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

impl Default for StrokeModeler {
    fn default() -> Self {
        let params = ModelerParams::suggested();

        Self {
            params,
            wobble_deque: VecDeque::with_capacity(
                (2.0 * params.sampling_min_output_rate * params.wobble_smoother_timeout) as usize,
            ),
            wobble_weighted_pos_sum: (0.0, 0.0),
            wobble_duration_sum: 0.0,
            wobble_distance_sum: 0.0,

            last_event: None,
            last_corrected_event: None,
            position_modeler: None,
            state_modeler: StateModeler::new(params.stylus_state_modeler_max_input_samples),
        }
    }
}
