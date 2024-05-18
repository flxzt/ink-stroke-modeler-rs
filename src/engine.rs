use std::vec;

use super::*;

use crate::utils::interp;
use crate::utils::normalize01_32;

/// smooth out the input position from high frequency noise
/// uses a moving average of position and interpolating between this
/// position and the raw position based on the speed.
/// high speeds movements won't be smoothed but low speed will.
///
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

#[doc = include_str!("../docs/notations.html")]
#[doc = include_str!("../docs/resampling.html")]
#[doc = include_str!("../docs/position_modeling.html")]
#[doc = include_str!("../docs/stylus_state_modeler.html")]
#[doc = include_str!("../docs/stroke_end.html")]
impl StrokeModeler {
    pub fn new(params: ModelerParams) -> Self {
        Self {
            params,
            last_event: None,
            last_corrected_event: None,
            wobble_decque: VecDeque::with_capacity(
                (2.0 * params.sampling_min_output_rate * params.wobble_smoother_timeout) as usize,
            ),
            wobble_duration_sum: 0.0,
            wobble_weighted_pos_sum: (0.0, 0.0),
            wobble_distance_sum: 0.0,
            position_modeler: None,
            state_modeler: StateModeler::default(),
        }
    }

    /// Clears any in-progress stroke, keeping the same model parameters
    /// Technically the error is not needed anymore as the params are kept
    /// as is and can't be uninitialized
    pub fn reset(&mut self) -> Result<(), i32> {
        self.last_event = None;
        self.wobble_decque = VecDeque::with_capacity(
            (2.0 * self.params.sampling_min_output_rate * self.params.wobble_smoother_timeout)
                as usize,
        );
        self.wobble_duration_sum = 0.0;
        self.wobble_weighted_pos_sum = (0.0, 0.0);
        self.position_modeler = None;
        Ok(()) // to match
    }

    /// Clears any in-progress stroke, and re initialize the model with
    /// the given parameters
    ///
    /// Here the error is also obsolete as the `ModelerParams` is expected
    /// to have been built with [ModelerParams::new] that validates the
    /// parameters
    pub fn reset_w_params(&mut self, params: ModelerParams) -> Result<(), i32> {
        match params.validate() {
            Ok(params) => self.params = params,
            Err(_) => return Err(0),
        };
        self.last_event = None;
        self.wobble_decque = VecDeque::with_capacity(
            (2.0 * self.params.sampling_min_output_rate * self.params.wobble_smoother_timeout)
                as usize,
        );
        self.wobble_duration_sum = 0.0;
        self.wobble_weighted_pos_sum = (0.0, 0.0);
        self.wobble_distance_sum = 0.0;
        self.position_modeler = None;
        Ok(())
    }

    /// Updates the model with a raw input, and appends newly generated Results to the results vector.
    /// Any previously generated Result values remain valid.
    /// (This does not require that any previous results returned remain in the results vector, as it is
    /// appended to without examining the existing contents)
    ///
    /// If this does not return an error, results will contain at least one Result, and potentially
    /// more if the inputs are slower than the minimum output rate
    ///
    /// for now rnote's wrapper codes verify that the input is not duplicated and time increases between strokes
    /// This is not tested here, as we suppose that these things are verified beforehand
    pub fn update(&mut self, input: ModelerInput) -> Result<Vec<ModelerResult>, i32> {
        // print to stdout the value (for raw values)
        // println!(
        //     "{:?};{:?};{:?};{:?}",
        //     &input.pos().0,
        //     &input.pos().1,
        //     &input.time,
        //     &input.pressure
        // );

        match input.event_type {
            ModelerInputEventType::kDown => {
                // assumed this is the first ever event
                self.wobble_update(&input); // first event is "as is"

                // create the position modeler
                self.position_modeler = Some(PositionModeler::new(self.params, input.clone()));

                self.last_event = Some(input.clone());
                self.last_corrected_event = Some(input.pos);
                self.state_modeler
                    .reset(self.params.stylus_state_modeler_max_input_samples);
                self.state_modeler.update(input.clone());
                Ok(vec![ModelerResult {
                    pos: input.pos,
                    velocity: (0.0, 0.0),
                    acceleration: (0.0, 0.0),
                    time: input.time,
                    pressure: input.pressure,
                }])
            }
            ModelerInputEventType::kMove => {
                // get the latest element
                if self.last_event.is_none() {
                    return Err(1);
                }
                let latest_time = self.last_event.as_ref().unwrap().time();
                let new_time = input.time();
                self.state_modeler.update(input.clone());

                // calculate the number of element to predict
                let n_tsteps = (((new_time - latest_time) * self.params.sampling_min_output_rate)
                    .ceil() as i32)
                    .min(i32::MAX);

                // this does not check for very large inputs
                // this does not error if the number of steps is larger than
                // [ModelParams::sampling_max_outputs_per_call]
                // normally there is some additional upsampling (see UpsampleDueToSharpTurn)
                // if the predicted velocity makes the stylus do a left or right turn
                // this was deactivated as the value was not set in the original `ink-stroke-modeler-rs`
                // but ofc this would also make the model output a larger number of elements ...

                let p_start = self.last_corrected_event.unwrap();
                let p_end = self.wobble_update(&input);
                // seems like speeds are way higher than normal speed encountered so no smoothing occurs here

                let vec_out: Vec<ModelerResult> = self
                    .position_modeler
                    .as_mut()
                    .unwrap()
                    .update_along_linear_path(p_start, latest_time, p_end, new_time, n_tsteps)
                    .into_iter()
                    .map(|i| ModelerResult {
                        pressure: self.state_modeler.query(i.pos),
                        pos: i.pos,
                        velocity: i.velocity,
                        acceleration: i.acceleration,
                        time: i.time,
                    })
                    .collect();

                // push the latest element (should we push everything we also interpolated as well ?)
                self.last_event = Some(input.clone());
                self.last_corrected_event = Some(p_end);

                Ok(vec_out)
            }
            ModelerInputEventType::kUp => {
                // get the latest element
                if self.last_event.is_none() {
                    return Err(1);
                }
                let latest_time = self.last_event.as_ref().unwrap().time;
                let new_time = input.time();
                self.state_modeler.update(input.clone());

                // calculate the number of element to predict
                let n_tsteps = (((new_time - latest_time) * self.params.sampling_min_output_rate)
                    .ceil() as i32)
                    .min(i32::MAX);

                let p_start = self.last_corrected_event.unwrap();
                // the p_end is purposefully different from the original implementation
                // to match the kMove part
                // the original takes the raw input here which means a different
                // behavior between the predict on a kMove and a kUp
                let p_end = self.wobble_update(&input);

                let mut vec_out = Vec::<ModelerResult>::with_capacity(
                    (n_tsteps as usize) + self.params.sampling_end_of_stroke_max_iterations,
                );

                vec_out.extend(
                    self.position_modeler
                        .as_mut()
                        .unwrap()
                        .update_along_linear_path(p_start, latest_time, p_end, new_time, n_tsteps)
                        .into_iter()
                        .map(|i| ModelerResult {
                            pressure: self.state_modeler.query(i.pos),
                            pos: i.pos,
                            velocity: i.velocity,
                            time: i.time,
                            acceleration: i.acceleration,
                        }),
                );

                // model the end of stroke
                vec_out.extend(
                    self.position_modeler
                        .as_mut()
                        .unwrap()
                        .model_end_of_stroke(
                            input.pos,
                            1. / self.params.sampling_min_output_rate,
                            self.params.sampling_end_of_stroke_max_iterations,
                            self.params.sampling_end_of_stroke_stopping_distance,
                        )
                        .into_iter()
                        .map(|i| ModelerResult {
                            pressure: self.state_modeler.query(i.pos),
                            pos: i.pos,
                            velocity: i.velocity,
                            acceleration: i.acceleration,
                            time: i.time,
                        }),
                );

                if vec_out.is_empty() {
                    let state_pos = self.position_modeler.as_ref().unwrap().state.clone();
                    vec_out.push(ModelerResult {
                        pos: state_pos.pos,
                        velocity: state_pos.velocity,
                        acceleration: state_pos.acceleration,
                        // this is so that the extra stroke added has a time that's larger than the previous one
                        // when the kUp happens at the same time as the kMove
                        // In the original implementation, this was always true because
                        // the ModelEndOfStroke function did not restore the state of the modeler
                        // so that even if a single candidate was tried and iterations stopped there
                        // the status of the modeler changed, including the time by at least
                        // `1. / self.params.sampling_min_output_rate`
                        time: state_pos.time + 1. / self.params.sampling_min_output_rate,
                        pressure: self.state_modeler.query(state_pos.pos),
                    });
                }

                // remove the last event
                self.last_event = None;

                Ok(vec_out)
            }
        }
    }

    /// Models the given input prediction without changing the internal model state
    ///
    /// Returns an error if the model has not yet been initialized,
    /// if there is no stroke in progress
    pub fn predict(&mut self) -> Result<Vec<ModelerResult>, String> {
        // for now return the latest element if it exists from the input
        if self.last_event.is_none() {
            // no data to predict from
            Err(String::from("empty input events"))
        } else {
            // construct the prediction (model_end_of_stroke does not modify the position modeler)
            let predict = self
                .position_modeler
                .as_mut()
                .unwrap()
                .model_end_of_stroke(
                    self.last_event.as_ref().unwrap().pos,
                    1. / self.params.sampling_min_output_rate,
                    self.params.sampling_end_of_stroke_max_iterations,
                    self.params.sampling_end_of_stroke_stopping_distance,
                )
                .into_iter()
                .map(|i| ModelerResult {
                    pos: i.pos,
                    velocity: i.velocity,
                    acceleration: i.acceleration,
                    time: i.time,
                    pressure: self.state_modeler.query(i.pos),
                })
                .collect();
            Ok(predict)
        }
    }
    ///implements the wobble logic
    ///smoothes out the input position from high frequency noise
    ///uses a moving average of position and interpolating between this
    ///position and the raw position based on the speed.
    ///high speeds movements won't be smoothed but low speed will.
    #[doc = include_str!("../docs/wobble.html")]
    pub fn wobble_update(&mut self, event: &ModelerInput) -> (f32, f32) {
        match self.wobble_decque.len() {
            0 => {
                self.wobble_decque.push_back(WobbleSample {
                    position: event.pos,
                    weighted_position: (0.0, 0.0),
                    distance: 0.0,
                    duration: 0.0,
                    time: event.time,
                });
                event.pos
            }
            _ => {
                let last_el = self.wobble_decque.back().unwrap();
                let duration = event.time - last_el.time;
                let weighted_pos = (event.pos.0 * duration as f32, event.pos.1 * duration as f32);
                let distance = ((event.pos.0 - last_el.position.0).powi(2)
                    + (event.pos.1 - last_el.position.1).powi(2))
                .sqrt();

                self.wobble_decque.push_back(WobbleSample {
                    position: event.pos,
                    weighted_position: weighted_pos,
                    distance,
                    duration,
                    time: event.time,
                });
                let last_pos = self.wobble_weighted_pos_sum;
                self.wobble_weighted_pos_sum =
                    (last_pos.0 + weighted_pos.0, last_pos.1 + weighted_pos.1);
                self.wobble_distance_sum += distance;
                self.wobble_duration_sum += duration;

                while self.wobble_decque.front().unwrap().time
                    < event.time - self.params.wobble_smoother_timeout
                {
                    let front_el = self.wobble_decque.pop_front().unwrap();

                    let last_pos = self.wobble_weighted_pos_sum;
                    self.wobble_weighted_pos_sum = (
                        last_pos.0 - front_el.weighted_position.0,
                        last_pos.1 - front_el.weighted_position.1,
                    );
                    self.wobble_distance_sum -= front_el.distance;
                    self.wobble_duration_sum -= front_el.duration;
                }

                if self.wobble_duration_sum < 1e-12 {
                    event.pos
                } else {
                    // calulate the average position

                    // weird f32 and f64 mix
                    let avg_position = (
                        self.wobble_weighted_pos_sum.0 / self.wobble_duration_sum as f32,
                        self.wobble_weighted_pos_sum.1 / self.wobble_duration_sum as f32,
                    );

                    let avg_speed = self.wobble_distance_sum / self.wobble_duration_sum as f32;
                    let norm_value = normalize01_32(
                        self.params.wobble_smoother_speed_floor,
                        self.params.wobble_smoother_speed_ceiling,
                        avg_speed,
                    );
                    (
                        interp(avg_position.0, event.pos.0, norm_value),
                        interp(avg_position.1, event.pos.1, norm_value),
                    )
                }
            }
        }
    }
}
