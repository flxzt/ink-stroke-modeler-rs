use std::vec;

use super::*;

use crate::utils::interp;
use crate::utils::normalize01_32;

impl StrokeModeler {
    pub fn new(params: ModelerParams) -> Self {
        Self {
            params,
            last_event: None,
            wobble_decque: VecDeque::new(),
            wobble_duration_sum: 0.0,
            wobble_weighted_pos_sum: (0.0, 0.0),
            wobble_distance_sum: 0.0,
            position_modeler: None,
            past_events: Vec::new(),
        }
    }

    /// Clears any in-progress stroke, keeping the same model parameters
    /// Technically the error is not needed anymore as the params are kept
    /// as is and can't be uninitialized
    pub fn reset(&mut self) -> Result<(), i32> {
        self.last_event = None;
        self.wobble_decque = VecDeque::new();
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
        self.params = params;
        self.last_event = None;
        self.wobble_decque = VecDeque::new();
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
    /// maybe better to change the API to have it not be copied everytime but using a reference ?
    /// As it's expected to be called 100s of times on a stroke, with a relatively small results part each time
    ///
    /// If this does not return an error, results will contain at least one Result, and potentially
    /// more if the inputs are slower than the minimum output rate
    ///
    /// for now rnote's wrapper codes verify that the input is not duplicated and time increases between strokes
    /// but this is tested in the ink-stroke-modeler code as well and would fail it
    ///
    /// Normally, there is a match on the kdown, kMove or KUp part done to process the event
    pub fn update(&mut self, input: ModelerInput) -> Result<Vec<ModelerInput>, i32> {
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
                self.position_modeler = Some(PositionModeler::new(self.params, input));

                self.last_event = Some(input);
                self.past_events.push(input);
                Ok(self.past_events.clone())
            }
            _ => {
                // get the latest element
                let latest_el = self.last_event.unwrap();
                let latest_time = latest_el.time();
                let new_time = input.time();

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

                let p_start = latest_el.pos();
                let p_end = self.wobble_update(&input);

                //println!("{:?}", (p_end.0-input.pos.0,p_end.1-input.pos.1)); //see if this has an effect
                // seems like speeds are way higher than normal speed encountered so no smoothing occurs here

                // there was an error with the last el not being taken with .. but is part of it with ..=
                let vec_out: Vec<ModelerInput> = self
                    .position_modeler
                    .as_mut()
                    .unwrap()
                    .update_along_linear_path(p_start, latest_time, p_end, new_time, n_tsteps)
                    .into_iter()
                    .map(|i| {
                        ModelerInput {
                            event_type: latest_el.event_type,
                            // for now we fake the pressure
                            pressure: input.pressure(),
                            pos: i.pos,
                            time: i.time,
                        }
                    })
                    .collect();

                // push the latest element (should we push everything we also interpolated as well ?)
                self.past_events.push(input);
                self.last_event = Some(input);

                Ok(vec_out)
            }
        }
    }

    /// Models the given input prediction without changing the internal model state
    ///
    /// and then (?) clears and fills the results parameters with the new predicted results ?
    /// Any previously generated prediction results are no longer valid
    ///
    /// Returns an error if the model has not yet been initialized,
    /// if there is no stroke in progress, or if prediction has been disabled
    ///
    /// The output is limited to results where the predictor has sufficient confidence
    ///
    /// results.clear ? on the start of this part of code ?
    /// TODO test that it does not work with [PredictionParams::Disabled]
    /// There should be a last input existing (maybe we should have it somewhere as a variable ?)
    pub fn predict(&mut self) -> Result<Vec<ModelerInput>, String> {
        // for now return the latest element if it exists from the input
        if self.past_events.is_empty()
            || self.params.prediction_params == PredictionParams::Disabled
        {
            Err(String::from("empty input events"))
        } else {
            let last_event = self.past_events.last().unwrap().clone();
            self.past_events = Vec::new(); //empty
            Ok(vec![last_event])
        }
    }
    ///implements the wobble logic
    ///smoothes out the input position from high frequency noise
    ///uses a moving average of position and interpolating between this
    ///position and the raw position based on the speed.
    ///high speeds movements won't be smoothed but low speed will.
    #[doc =include_str!("./wobble_doc.html")]
    pub fn wobble_update(&mut self, event: &ModelerInput) -> (f32, f32) {
        //println!("len wobble decque {:?}",self.wobble_decque.len());
        match self.wobble_decque.len() {
            0 => {
                self.wobble_decque.push_back(WobbleSample {
                    position: event.pos,
                    weighted_position: (0.0, 0.0),
                    distance: 0.0,
                    duration: 0.0,
                    time: event.time,
                });
                return event.pos;
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
                    distance: distance,
                    duration: duration,
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
                    //println!("pop");
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
                    return event.pos;
                } else {
                    //println!("whole decque {:?}",self.wobble_decque);
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
                    //println!("avg position : {:?}, true position : {:?}, norm value : {:?}, speed : {:?}",avg_position,event.pos,norm_value,avg_speed);
                    return (
                        interp(avg_position.0, event.pos.0, norm_value),
                        interp(avg_position.1, event.pos.1, norm_value),
                    );
                }
            }
        }
    }
}
