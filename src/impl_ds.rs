// import
use super::*;

// implementation for all structures

/// To create a new input
///
/// This is kinda superfluous because initializattion as a struct can also be done
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
    }

    // helper methods : superfluous if we make the params of the struct public
    // we don't have to do a ffi unwrap

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

    pub fn near(self, other: ModelerResult) -> bool {
        let tol = 1e-4;
        ((self.pos.0 - other.pos.0).abs() < tol
            && (self.pos.1 - other.pos.1).abs() < tol
            && (self.time - other.time).abs() < tol as f64)
            && (self.acceleration.0 - other.acceleration.0).abs() < tol
            && (self.acceleration.1 - other.acceleration.1).abs() < tol
            && (self.velocity.0 - other.velocity.0).abs() < tol
            && (self.velocity.1 - other.velocity.1).abs() < tol
            && (self.pressure - other.pressure).abs() < tol
    }
}

impl Default for ModelerResult {
    fn default() -> Self {
        Self {
            pos: (0.0, 0.0),
            velocity: (0.0, 0.0),
            acceleration: (0.0, 0.0),
            pressure: 1.0,
            time: 0.0,
        }
    }
}
/// Utility to compare [ModelerResult] up to `tol =1e-4` (not settable for now)
/// with printed output for debug purposes
pub fn compare_results(left: Vec<ModelerResult>, right: Vec<ModelerResult>) -> bool {
    if left.len() != right.len() {
        tracing::debug!("\n\nleft : {:?} right {:?}", left.len(), right.len());
        //iterate
        tracing::debug!("left");
        for el in left {
            tracing::debug!("{:?}", el);
        }
        tracing::debug!("right");
        for el in right {
            tracing::debug!("{:?}", el);
        }
        false
    } else {
        tracing::debug!("\n\n\nleft : {:?} right {:?}", &left.len(), &right.len());
        //iterate
        tracing::debug!("left");
        for el in &left {
            tracing::debug!("{:?}", el);
        }
        tracing::debug!("right");
        for el in &right {
            tracing::debug!("{:?}", el);
        }

        left.into_iter().zip(right).all(|x| x.0.near(x.1))
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
            state_modeler: StateModeler::default(),
        }
    }
}
