// import
use super::*;

// implementation for all structures

// for now most parameters won't be used
impl KalmanPredictorParams {
    /// returns a suggested tuning for the kalman predictor
    /// - `process_noise`: `1.0`,
    /// - `measurement_noise`: `1.0`,
    /// - `min_stable_iteration`: `4,`
    /// - `max_time_samples`: `20`,
    /// - `min_catchup_velocity`: `0.02`,
    /// - `acceleration_weight`: `0.5`,
    /// - `jerk_weight`: `0.1`,
    /// - `prediction_interval`: `0.02`,
    /// - `confidence_desired_number_of_samples`: `20`,
    /// - `confidence_max_estimation_distance`: `1.5`,
    /// - `confidence_min_travel_speed`: `1.0`,
    /// - `confidence_max_travel_speed`: `5.0`,
    /// - `confidence_max_linear_deviation`: `10.0`,
    /// - `confidence_baseline_linearity_confidence`: `0.4`,
    pub fn suggested() -> Self {
        Self {
            process_noise: 1.0,
            measurement_noise: 1.0,
            min_stable_iteration: 4,
            max_time_samples: 20,
            min_catchup_velocity: 0.02,
            acceleration_weight: 0.5,
            jerk_weight: 0.1,
            prediction_interval: 0.02,
            confidence_desired_number_of_samples: 20,
            confidence_max_estimation_distance: 1.5,
            confidence_min_travel_speed: 1.0,
            confidence_max_travel_speed: 1.5,
            confidence_max_linear_deviation: 10.0,
            confidence_baseline_linearity_confidence: 0.4,
        }
    }

    pub fn new(
        process_noise: f64,
        measurement_noise: f64,
        min_stable_iteration: i32,
        max_time_samples: i32,
        min_catchup_velocity: f32,
        acceleration_weight: f32,
        jerk_weight: f32,
        prediction_interval: f64,
        confidence_desired_number_of_samples: i32,
        confidence_max_estimation_distance: f32,
        confidence_min_travel_speed: f32,
        confidence_max_travel_speed: f32,
        confidence_max_linear_deviation: f32,
        confidence_baseline_linearity_confidence: f32,
    ) -> Result<Self, String> {
        let tests = [
            process_noise > 0.0,
            measurement_noise > 0.0,
            min_stable_iteration > 0,
            max_time_samples > 0,
            min_catchup_velocity > 0.0,
            acceleration_weight.is_finite(),
            jerk_weight.is_finite(),
            prediction_interval > 0.0,
            confidence_desired_number_of_samples > 0,
            confidence_max_estimation_distance > 0.0,
            confidence_min_travel_speed > 0.0,
            confidence_max_travel_speed > 0.0,
            confidence_min_travel_speed < confidence_max_travel_speed,
            confidence_max_linear_deviation > 0.0,
            (0.0..=1.0).contains(&confidence_baseline_linearity_confidence),
        ];

        match tests.iter().fold(true, |acc, x| acc & x) {
            true => Ok(Self {
                process_noise,
                measurement_noise,
                min_stable_iteration,
                max_time_samples,
                min_catchup_velocity,
                acceleration_weight,
                jerk_weight,
                prediction_interval,
                confidence_desired_number_of_samples,
                confidence_max_estimation_distance,
                confidence_min_travel_speed,
                confidence_max_travel_speed,
                confidence_max_linear_deviation,
                confidence_baseline_linearity_confidence,
            }),
            false => {
                let error_messages = [
                    "`process_noise` should be positive; ",
                    "`measurement_noise` should be positive; ",
                    "`min_stable_iteration` should be positive; ",
                    "`max_time_samples` should be positive; ",
                    "`min_catchup_velocity` should be positive; ",
                    "`acceleration_weight` should be finite; ",
                    "`jerk_weight` should be finite; ",
                    "`prediction_interval` should be positive; ",
                    "`confidence_desired_number_of_samples` should be positive; ",
                    "`confidence_max_estimation_distance` should be positive; ",
                    "`confidence_min_travel_speed` should be positive; ",
                    "`confidence_max_travel_speed` should be positive; ",
                    "`confidence_min_travel_speed` should be < to `confidence_max_travel_speed`; ",
                    "`confidence_max_linear_deviation` should be positive; ",
                    "`confidence_baseline_linearity_confidence` should be in the interval [0,1]; ",
                ];

                //Collect errors
                let error_acc = tests
                    .iter()
                    .zip(error_messages)
                    .filter(|x| !*(x.0))
                    .fold(String::from("the following errors occured : "), |acc, x| {
                        acc + x.1
                    });

                Err(error_acc)
            }
        }
    }
}

// impl for ModelerInput
impl ModelerInput {
    pub fn new(
        event_type: ModelerInputEventType,
        pos: (f32, f32),
        time: f64,
        pressure: f32,
    ) -> Self {
        ModelerInput {
            event_type,
            pos,
            time,
            pressure,
        }
        // probably needs to be changed because of mutability and access
    }

    // helper methods : still useful for compat + mutability
    pub fn event_type(&self) -> ModelerInputEventType {
        self.event_type
    }

    pub fn pos(&self) -> (f32, f32) {
        self.pos
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn pressure(&self) -> f32 {
        self.pressure
    }
}

//
impl ModelerParams {
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
            stylus_state_modeler_max_input_samples: 100_000,
            prediction_params: PredictionParams::StrokeEnd,
        }
    }

    pub fn new(
        wobble_smoother_timeout: f64,
        wobble_smoother_speed_floor: f32,
        wobble_smoother_speed_ceiling: f32,
        position_modeler_spring_mass_constant: f32,
        position_modeler_drag_constant: f32,
        sampling_min_output_rate: f64,
        sampling_end_of_stroke_stopping_distance: f32,
        sampling_end_of_stroke_max_iterations: i32,
        sampling_max_outputs_per_call: i32,
        stylus_state_modeler_max_input_samples: i32,
        prediction_params: PredictionParams,
    ) -> Result<Self, String> {
        let parameter_tests = [
            position_modeler_spring_mass_constant > 0.0,
            position_modeler_drag_constant > 0.0,
            sampling_min_output_rate > 0.0,
            sampling_end_of_stroke_stopping_distance > 0.0,
            sampling_end_of_stroke_max_iterations > 0,
            sampling_end_of_stroke_max_iterations < 1000,
            sampling_max_outputs_per_call > 0,
            wobble_smoother_timeout > 0.0,
            wobble_smoother_speed_floor > 0.0,
            wobble_smoother_speed_ceiling > 0.0,
            wobble_smoother_speed_floor < wobble_smoother_speed_ceiling,
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
            Ok(Self {
                wobble_smoother_timeout,
                wobble_smoother_speed_floor,
                wobble_smoother_speed_ceiling,
                position_modeler_spring_mass_constant,
                position_modeler_drag_constant,
                sampling_min_output_rate,
                sampling_end_of_stroke_stopping_distance,
                sampling_end_of_stroke_max_iterations,
                sampling_max_outputs_per_call,
                stylus_state_modeler_max_input_samples,
                prediction_params,
            })
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

impl ModelerResult {
    // not too needed, self.pos vs self.pos() ...
    pub fn pos(&self) -> (f32, f32) {
        self.pos
    }

    pub fn velocity(&self) -> (f32, f32) {
        self.velocity
    }

    pub fn acceleration(&self) -> (f32, f32) {
        self.acceleration
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn pressure(&self) -> f32 {
        self.pressure
    }
}

impl Default for StrokeModeler {
    fn default() -> Self {
        let params = ModelerParams::suggested();

        Self {
            params,
            wobble_decque: VecDeque::new(),
            wobble_weighted_pos_sum: (0.0, 0.0),
            wobble_duration_sum: 0.0,
            wobble_distance_sum: 0.0,

            last_event: None,
            position_modeler: None,
            past_events: Vec::new(),
        }
    }
}
