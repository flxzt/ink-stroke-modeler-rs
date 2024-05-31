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
pub(crate) struct WobbleSample {
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
    pub(crate) params: ModelerParams,
    /// wobble smoother structures
    /// deque to hold events that are recent
    /// to calculate a moving average
    pub(crate) wobble_deque: VecDeque<WobbleSample>,
    /// running weighted sum
    pub(crate) wobble_weighted_pos_sum: (f32, f32),
    /// running duration sum
    pub(crate) wobble_duration_sum: f64,
    /// running distance sum
    pub(crate) wobble_distance_sum: f32,
    // physical model for the stroke
    // only created on the first stroke
    pub(crate) position_modeler: Option<PositionModeler>,
    pub(crate) last_event: Option<ModelerInput>,
    pub(crate) last_corrected_event: Option<(f32, f32)>,
    pub(crate) state_modeler: StateModeler,
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

#[doc = include_str!("../docs/notations.html")]
#[doc = include_str!("../docs/resampling.html")]
#[doc = include_str!("../docs/position_modeling.html")]
#[doc = include_str!("../docs/stylus_state_modeler.html")]
#[doc = include_str!("../docs/stroke_end.html")]
impl StrokeModeler {
    pub fn new(params: ModelerParams) -> Result<Self, String> {
        params.validate()?;
        Ok(Self {
            params,
            last_event: None,
            last_corrected_event: None,
            wobble_deque: VecDeque::with_capacity(
                (2.0 * params.sampling_min_output_rate * params.wobble_smoother_timeout) as usize,
            ),
            wobble_duration_sum: 0.0,
            wobble_weighted_pos_sum: (0.0, 0.0),
            wobble_distance_sum: 0.0,
            position_modeler: None,
            state_modeler: StateModeler::new(params.stylus_state_modeler_max_input_samples),
        })
    }

    /// Clears any in-progress stroke, keeping the same model parameters
    pub fn reset(&mut self) {
        self.last_event = None;
        self.wobble_deque = VecDeque::with_capacity(
            (2.0 * self.params.sampling_min_output_rate * self.params.wobble_smoother_timeout)
                as usize,
        );
        self.wobble_duration_sum = 0.0;
        self.wobble_weighted_pos_sum = (0.0, 0.0);
        self.position_modeler = None;
        self.state_modeler
            .reset(self.params.stylus_state_modeler_max_input_samples);
    }

    /// Clears any in-progress stroke, and re initialize the model with
    /// the given parameters
    pub fn reset_w_params(&mut self, params: ModelerParams) -> Result<(), String> {
        params.validate()?;
        self.last_event = None;
        self.wobble_deque = VecDeque::with_capacity(
            (2.0 * params.sampling_min_output_rate * params.wobble_smoother_timeout) as usize,
        );
        self.wobble_duration_sum = 0.0;
        self.wobble_weighted_pos_sum = (0.0, 0.0);
        self.wobble_distance_sum = 0.0;
        self.position_modeler = None;
        self.state_modeler
            .reset(params.stylus_state_modeler_max_input_samples);
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
    pub fn update(&mut self, input: ModelerInput) -> Result<Vec<ModelerResult>, String> {
        match input.event_type {
            ModelerInputEventType::Down => {
                if self.last_event.is_some() {
                    return Err(String::from("down event is not the first event or a down event occured after another one"));
                }
                self.wobble_update(&input); // first event is "as is"

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
            ModelerInputEventType::Move => {
                // get the latest element
                if self.last_event.is_none() {
                    return Err(String::from("no Down event occurred before a Move event"));
                }
                let latest_time = self.last_event.as_ref().unwrap().time;
                let new_time = input.time;
                self.state_modeler.update(input.clone());

                // calculate the number of element to predict
                let n_tsteps = (((new_time - latest_time) * self.params.sampling_min_output_rate)
                    .ceil() as i32)
                    .min(i32::MAX);

                // this does not check for very large inputs
                // this does not error if the number of steps is larger than
                // [ModelParams::sampling_max_outputs_per_call]

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
            ModelerInputEventType::Up => {
                // get the latest element
                if self.last_event.is_none() {
                    return Err(String::from("No event occured before an up event"));
                }
                let latest_time = self.last_event.as_ref().unwrap().time;
                let new_time = input.time;
                self.state_modeler.update(input.clone());

                // calculate the number of element to predict
                let n_tsteps = (((new_time - latest_time) * self.params.sampling_min_output_rate)
                    .ceil() as i32)
                    .min(i32::MAX);

                let p_start = self.last_corrected_event.unwrap();
                // the p_end is purposefully different from the original implementation
                // to match the Move part
                // the original takes the raw input here which means a different
                // behavior between the predict on a Move and a Up
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
                        // when the Up happens at the same time as the Move
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
    fn wobble_update(&mut self, event: &ModelerInput) -> (f32, f32) {
        match self.wobble_deque.len() {
            0 => {
                self.wobble_deque.push_back(WobbleSample {
                    position: event.pos,
                    weighted_position: (0.0, 0.0),
                    distance: 0.0,
                    duration: 0.0,
                    time: event.time,
                });
                event.pos
            }
            _ => {
                let last_el = self.wobble_deque.back().unwrap();
                let duration = event.time - last_el.time;
                let weighted_pos = (event.pos.0 * duration as f32, event.pos.1 * duration as f32);
                let distance = ((event.pos.0 - last_el.position.0).powi(2)
                    + (event.pos.1 - last_el.position.1).powi(2))
                .sqrt();

                self.wobble_deque.push_back(WobbleSample {
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

                while self.wobble_deque.front().unwrap().time
                    < event.time - self.params.wobble_smoother_timeout
                {
                    let front_el = self.wobble_deque.pop_front().unwrap();

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

#[cfg(test)]
mod tests {

    use super::super::*;
    use crate::results::compare_results;

    /// compare (f32,f32) floats up to `0.0001` precision
    /// utility for testing only
    #[cfg(test)]
    fn util_compare_floats(a1: (f32, f32), a2: (f32, f32)) -> bool {
        return approx::abs_diff_eq!(a1.0, a2.0, epsilon = 0.0001)
            && approx::abs_diff_eq!(a1.1, a2.1, epsilon = 0.0001);
    }

    // wobble tests

    #[test]
    fn test_wobble_smoother_line() {
        // need to create a StrokeModeler
        let mut new_modeler = StrokeModeler::default();
        new_modeler.wobble_update(&ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (3., 4.),
            time: 1.0,
            pressure: 0.0,
        });
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (3.016, 4.),
                time: 1.016,
                pressure: 0.0,
            }),
            (3.016, 4.)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (3.032, 4.),
                time: 1.032,
                pressure: 0.0,
            }),
            (3.024, 4.)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (3.048, 4.),
                time: 1.048,
                pressure: 0.0,
            }),
            (3.032, 4.)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (3.064, 4.),
                time: 1.064,
                pressure: 0.0,
            }),
            (3.048, 4.)
        ));
    }

    #[test]
    fn test_wobble_zigzag_slow() {
        // need to create a StrokeModeler
        let mut new_modeler = StrokeModeler::default();
        new_modeler.wobble_update(&ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (1., 2.),
            time: 5.0,
            pressure: 0.0,
        });
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (1.016, 2.),
                time: 5.016,
                pressure: 0.0,
            }),
            (1.016, 2.0)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (1.016, 2.016),
                time: 5.032,
                pressure: 0.0,
            }),
            (1.016, 2.008)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (1.032, 2.016),
                time: 5.048,
                pressure: 0.0,
            }),
            (1.02133, 2.01067)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (1.032, 2.032),
                time: 5.064,
                pressure: 0.0,
            }),
            (1.0266667, 2.0213333)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (1.048, 2.032),
                time: 5.080,
                pressure: 0.0,
            }),
            (1.0373333, 2.0266667)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (1.048, 2.048),
                time: 5.096,
                pressure: 0.0,
            }),
            (1.0426667, 2.0373333)
        ));
    }

    #[test]
    fn fast_zigzag() {
        let mut new_modeler = StrokeModeler::default();
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (7., 3.024),
                time: 8.016,
                pressure: 0.0,
            }),
            (7.0, 3.024)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (7.024, 3.024),
                time: 8.032,
                pressure: 0.0,
            }),
            (7.024, 3.024)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (7.024, 3.048),
                time: 8.048,
                pressure: 0.0,
            }),
            (7.024, 3.048)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (7.048, 3.048),
                time: 8.064,
                pressure: 0.0,
            }),
            (7.048, 3.048)
        ));
    }

    #[test]
    fn input_test() {
        let mut modeler = StrokeModeler::new(ModelerParams::suggested()).unwrap();

        let inputs = vec![
            ModelerInput {
                event_type: ModelerInputEventType::Down,
                pos: (0.0, 0.0),
                time: 0.0,
                pressure: 0.1,
            },
            ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (1.0, 0.0),
                time: 0.02,
                pressure: 0.3,
            },
            ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (2.0, 0.0),
                time: 0.04,
                pressure: 0.5,
            },
            ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (2.5, 1.0),
                time: 0.06,
                pressure: 0.8,
            },
            ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (3.0, 1.5),
                time: 0.12,
                pressure: 0.9,
            },
            ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (4.0, 2.0),
                time: 0.13,
                pressure: 0.8,
            },
            ModelerInput {
                event_type: ModelerInputEventType::Move,
                pos: (3.8, 2.1),
                time: 0.14,
                pressure: 0.7,
            },
            ModelerInput {
                event_type: ModelerInputEventType::Up,
                pos: (3.5, 2.0),
                time: 0.14,
                pressure: 0.2,
            },
        ];

        for res in inputs.into_iter().flat_map(|i| modeler.update(i)) {
            println!("{res:?}");
        }
    }

    //tests for the end of stroke prediction
    #[test]
    fn test_empty_prediction() {
        let mut engine = StrokeModeler::new(ModelerParams::suggested()).unwrap();
        assert!(engine.predict().is_err());
    }

    #[test]
    fn test_singleinput() {
        let mut engine = StrokeModeler::default();
        engine
            .update(ModelerInput {
                pos: (4.0, 5.0),
                event_type: ModelerInputEventType::Down,
                time: 2.0,
                pressure: 1.0,
            })
            .unwrap();
        assert_eq!(engine.predict().unwrap().len(), 0);
    }

    // tests for the stroke modeler
    #[test]
    fn input_rate_slower() {
        // ceil is exactly on the limit making it different from the original test without this cast
        let delta_time = (1. / 30. as f32) as f64;
        let mut time = 0.0;
        let mut engine = StrokeModeler::new(ModelerParams {
            stylus_state_modeler_max_input_samples: 20,
            ..ModelerParams::suggested()
        })
        .unwrap();

        let first_iter = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (3., 4.),
            time: time,
            pressure: 1.0,
        });
        assert!(first_iter.is_ok());
        assert!(compare_results(
            first_iter.unwrap(),
            vec![ModelerResult {
                pos: (3.0, 4.0),
                ..ModelerResult::default()
            }]
        ));

        assert!(engine.predict().is_ok());
        assert!(engine.predict().unwrap().is_empty());

        time += delta_time;
        assert!(compare_results(
            engine
                .update(ModelerInput {
                    event_type: ModelerInputEventType::Move,
                    pos: (3.2, 4.2),
                    time: time,
                    pressure: 1.0
                })
                .unwrap(),
            vec![
                ModelerResult {
                    pos: (3.0019, 4.0019),
                    velocity: (0.4007, 0.4007),
                    acceleration: (84.1557, 84.1564),
                    time: 0.0048,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.0069, 4.0069),
                    velocity: (1.0381, 1.0381),
                    acceleration: (133.8378, 133.8369),
                    time: 0.0095,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.0154, 4.0154),
                    velocity: (1.7883, 1.7883),
                    acceleration: (157.5465, 157.5459),
                    time: 0.0143,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.0276, 4.0276),
                    velocity: (2.5626, 2.5626),
                    acceleration: (162.6039, 162.6021),
                    time: 0.0190,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.0433, 4.0433),
                    velocity: (3.3010, 3.3010),
                    acceleration: (155.0670, 155.0666),
                    time: 0.0238,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.0622, 4.0622),
                    velocity: (3.9665, 3.9665),
                    acceleration: (139.7575, 139.7564),
                    time: 0.0286,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.0838, 4.0838),
                    velocity: (4.5397, 4.5397),
                    acceleration: (120.3618, 120.3625),
                    time: 0.0333,
                    pressure: 1.0
                }
            ]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (3.1095, 4.1095),
                    velocity: (4.6253, 4.6253),
                    acceleration: (15.4218, 15.4223),
                    time: 0.0389,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1331, 4.1331),
                    velocity: (4.2563, 4.2563),
                    acceleration: (-66.4341, -66.4339),
                    time: 0.0444,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1534, 4.1534),
                    velocity: (3.6479, 3.6479),
                    acceleration: (-109.5083, -109.5081),
                    time: 0.0500,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1698, 4.1698),
                    velocity: (2.9512, 2.9512),
                    acceleration: (-125.3978, -125.3976),
                    time: 0.0556,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1824, 4.1824),
                    velocity: (2.2649, 2.2649),
                    acceleration: (-123.5318, -123.5310),
                    time: 0.0611,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1915, 4.1915),
                    velocity: (1.6473, 1.6473),
                    acceleration: (-111.1818, -111.1806),
                    time: 0.0667,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1978, 4.1978),
                    velocity: (1.1269, 1.1269),
                    acceleration: (-93.6643, -93.6636),
                    time: 0.0722,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1992, 4.1992),
                    velocity: (1.0232, 1.0232),
                    acceleration: (-74.6390, -74.6392),
                    time: 0.0736,
                    pressure: 1.0
                }
            ]
        ));

        time += delta_time;
        let second_results = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (3.5, 4.2),
            time: time,
            pressure: 1.0,
        });
        assert!(second_results.is_ok());
        assert!(compare_results(
            second_results.unwrap(),
            vec![
                ModelerResult {
                    pos: (3.1086, 4.1058),
                    velocity: (5.2142, 4.6131),
                    acceleration: (141.6557, 15.4223),
                    time: 0.0381,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1368, 4.1265),
                    velocity: (5.9103, 4.3532),
                    acceleration: (146.1873, -54.5680),
                    time: 0.0429,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.1681, 4.1450),
                    velocity: (6.5742, 3.8917),
                    acceleration: (139.4012, -96.9169),
                    time: 0.0476,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.2022, 4.1609),
                    velocity: (7.1724, 3.3285),
                    acceleration: (125.6306, -118.2742),
                    time: 0.0524,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.2388, 4.1739),
                    velocity: (7.6876, 2.7361),
                    acceleration: (108.1908, -124.4087),
                    time: 0.0571,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.2775, 4.1842),
                    velocity: (8.1138, 2.1640),
                    acceleration: (89.5049, -120.1309),
                    time: 0.0619,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.3177, 4.1920),
                    velocity: (8.4531, 1.6436),
                    acceleration: (71.2473, -109.2959),
                    time: 0.0667,
                    pressure: 1.0
                }
            ]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (3.3625, 4.1982),
                    velocity: (8.0545, 1.1165),
                    acceleration: (-71.7427, -94.8765),
                    time: 0.0722,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4018, 4.2021),
                    velocity: (7.0831, 0.6987),
                    acceleration: (-174.8469, -75.1957),
                    time: 0.0778,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4344, 4.2043),
                    velocity: (5.8564, 0.3846),
                    acceleration: (-220.8140, -56.5515),
                    time: 0.0833,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4598, 4.2052),
                    velocity: (4.5880, 0.1611),
                    acceleration: (-228.3204, -40.2244),
                    time: 0.0889,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4788, 4.2052),
                    velocity: (3.4098, 0.0124),
                    acceleration: (-212.0678, -26.7709),
                    time: 0.0944,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4921, 4.2048),
                    velocity: (2.3929, -0.0780),
                    acceleration: (-183.0373, -16.2648),
                    time: 0.1000,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4976, 4.2045),
                    velocity: (1.9791, -0.1015),
                    acceleration: (-148.9792, -8.4822),
                    time: 0.1028,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.5001, 4.2044),
                    velocity: (1.7911, -0.1098),
                    acceleration: (-135.3759, -5.9543),
                    time: 0.1042,
                    pressure: 1.0
                }
            ]
        ));

        // we get more strokes as the model catches up to the anchor postion
        time += delta_time;
        let update = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Up,
            pos: (3.7, 4.4),
            time: time,
            pressure: 1.0,
        });
        assert!(update.is_ok());
        assert!(compare_results(
            update.unwrap(),
            vec![
                ModelerResult {
                    pos: (3.3583, 4.1996),
                    velocity: (8.5122, 1.5925),
                    acceleration: (12.4129, -10.7201),
                    time: 0.0714,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.3982, 4.2084),
                    velocity: (8.3832, 1.8534),
                    acceleration: (-27.0783, 54.7731),
                    time: 0.0762,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4369, 4.2194),
                    velocity: (8.1393, 2.3017),
                    acceleration: (-51.2222, 94.1542),
                    time: 0.0810,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.4743, 4.2329),
                    velocity: (7.8362, 2.8434),
                    acceleration: (-63.6668, 113.7452),
                    time: 0.0857,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.5100, 4.2492),
                    velocity: (7.5143, 3.4101),
                    acceleration: (-67.5926, 119.0224),
                    time: 0.0905,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.5443, 4.2680),
                    velocity: (7.2016, 3.9556),
                    acceleration: (-65.6568, 114.5394),
                    time: 0.0952,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.5773, 4.2892),
                    velocity: (6.9159, 4.4505),
                    acceleration: (-59.9999, 103.9444),
                    time: 0.1000,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.6115, 4.3141),
                    velocity: (6.1580, 4.4832),
                    acceleration: (-136.4312, 5.8833),
                    time: 0.1056,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.6400, 4.3369),
                    velocity: (5.1434, 4.0953),
                    acceleration: (-182.6254, -69.8314),
                    time: 0.1111,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.6626, 4.3563),
                    velocity: (4.0671, 3.4902),
                    acceleration: (-193.7401, -108.9119),
                    time: 0.1167,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.6796, 4.3719),
                    velocity: (3.0515, 2.8099),
                    acceleration: (-182.7957, -122.4598),
                    time: 0.1222,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.6916, 4.3838),
                    velocity: (2.1648, 2.1462),
                    acceleration: (-159.6116, -119.4551),
                    time: 0.1278,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.6996, 4.3924),
                    velocity: (1.4360, 1.5529),
                    acceleration: (-131.1906, -106.7926),
                    time: 0.1333,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (3.7028, 4.3960),
                    velocity: (1.1520, 1.3044),
                    acceleration: (-102.2117, -89.4872),
                    time: 0.1361,
                    pressure: 1.0
                }
            ]
        ));

        // the stroke is finised, we get an error if we predict it
        assert!(engine.predict().is_err());
    }

    #[test]
    fn reset_keep_params() {
        let input = ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (3.0, 4.0),
            time: 0.0,
            pressure: 1.0,
        };
        let mut engine = StrokeModeler::default();
        assert!(engine.update(input.clone()).is_ok());

        assert!(engine.reset_w_params(ModelerParams::suggested()).is_ok());
        assert!(engine.update(input.clone()).is_ok());
    }

    /// InputRateFasterThanMinOutputRate
    #[test]
    fn input_rate_faster() {
        let delta_time = 1. / 300.;
        let mut engine = StrokeModeler::default();

        let mut time = 2.0;

        let res1 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (5.0, -3.0),
            time: time,
            pressure: 1.0,
        });

        assert!(res1.is_ok());
        assert!(compare_results(
            res1.unwrap(),
            vec![ModelerResult {
                pos: (5.0, -3.0),
                time: time,
                velocity: (0.0, 0.0),
                acceleration: (0.0, 0.0),
                pressure: 1.0
            }]
        ));

        assert!(engine.predict().is_ok());
        assert!(engine.predict().unwrap().is_empty());

        time += delta_time;
        let res2 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (5.0, -3.1),
            time: time,
            pressure: 1.0,
        });
        assert!(res2.is_ok());
        assert!(compare_results(
            res2.unwrap(),
            vec![ModelerResult {
                pos: (5.0, -3.0033),
                velocity: (0.0, -0.9818),
                acceleration: (0.0, -294.5452),
                time: 2.0033,
                pressure: 1.0
            }]
        ));

        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (5.0, -3.0153),
                    velocity: (0.0, -2.1719),
                    acceleration: (0.0, -214.2145),
                    time: 2.0089,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0303),
                    velocity: (0.0, -2.6885),
                    acceleration: (0.0, -92.9885),
                    time: 2.0144,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0456),
                    velocity: (0.0, -2.7541),
                    acceleration: (0.0, -11.7992),
                    time: 2.0200,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0597),
                    velocity: (0.0, -2.5430),
                    acceleration: (0.0, 37.9868),
                    time: 2.0256,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0718),
                    velocity: (0.0, -2.1852),
                    acceleration: (0.0, 64.4053),
                    time: 2.0311,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0817),
                    velocity: (0.0, -1.7719),
                    acceleration: (0.0, 74.4011),
                    time: 2.0367,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0893),
                    velocity: (0.0, -1.3628),
                    acceleration: (0.0, 73.6345),
                    time: 2.0422,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0948),
                    velocity: (0.0, -0.9934),
                    acceleration: (0.0, 66.4807),
                    time: 2.0478,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (5.0, -3.0986),
                    velocity: (0.0, -0.6815),
                    acceleration: (0.0, 56.1448),
                    time: 2.0533,
                    pressure: 1.0
                }
            ]
        ));
        assert!(engine.predict().is_ok());

        time += delta_time;

        let res3 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (4.975, -3.175),
            time: time,
            pressure: 1.0,
        });
        assert!(res3.is_ok());
        assert!(compare_results(
            res3.unwrap(),
            vec![ModelerResult {
                pos: (4.9992, -3.0114),
                velocity: (-0.2455, -2.4322),
                acceleration: (-73.6366, -435.1238),
                time: 2.0067,
                pressure: 1.0
            }]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (4.9962, -3.0344),
                    velocity: (-0.5430, -4.1368),
                    acceleration: (-53.5537, -306.8140),
                    time: 2.0122,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9924, -3.0609),
                    velocity: (-0.6721, -4.7834),
                    acceleration: (-23.2474, -116.3963),
                    time: 2.0178,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9886, -3.0873),
                    velocity: (-0.6885, -4.7365),
                    acceleration: (-2.9498, 8.4358),
                    time: 2.0233,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9851, -3.1110),
                    velocity: (-0.6358, -4.2778),
                    acceleration: (9.4971, 82.5682),
                    time: 2.0289,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9820, -3.1311),
                    velocity: (-0.5463, -3.6137),
                    acceleration: (16.1014, 119.5413),
                    time: 2.0344,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9796, -3.1471),
                    velocity: (-0.4430, -2.8867),
                    acceleration: (18.6005, 130.8578),
                    time: 2.0400,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9777, -3.1593),
                    velocity: (-0.3407, -2.1881),
                    acceleration: (18.4089, 125.7516),
                    time: 2.0456,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9763, -3.1680),
                    velocity: (-0.2484, -1.5700),
                    acceleration: (16.6198, 111.2560),
                    time: 2.0511,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9754, -3.1739),
                    velocity: (-0.1704, -1.0564),
                    acceleration: (14.0365, 92.4447),
                    time: 2.0567,
                    pressure: 1.0
                }
            ]
        ));

        time += delta_time;
        let res4 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (4.9, -3.2),
            time: time,
            pressure: 1.0,
        });
        assert!(res4.is_ok());
        assert!(compare_results(
            res4.unwrap(),
            vec![ModelerResult {
                pos: (4.9953, -3.0237),
                velocity: (-1.1603, -3.7004),
                acceleration: (-274.4622, -380.4507),
                time: 2.0100,
                pressure: 1.0
            }]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (4.9828, -3.0521),
                    velocity: (-2.2559, -5.1049),
                    acceleration: (-197.1994, -252.8115),
                    time: 2.0156,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9677, -3.0825),
                    velocity: (-2.7081, -5.4835),
                    acceleration: (-81.4051, -68.1520),
                    time: 2.0211,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9526, -3.1115),
                    velocity: (-2.7333, -5.2122),
                    acceleration: (-4.5282, 48.8396),
                    time: 2.0267,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9387, -3.1369),
                    velocity: (-2.4999, -4.5756),
                    acceleration: (42.0094, 114.5943),
                    time: 2.0322,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9268, -3.1579),
                    velocity: (-2.1326, -3.7776),
                    acceleration: (66.1132, 143.6292),
                    time: 2.0378,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9173, -3.1743),
                    velocity: (-1.7184, -2.9554),
                    acceleration: (74.5656, 147.9932),
                    time: 2.0433,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9100, -3.1865),
                    velocity: (-1.3136, -2.1935),
                    acceleration: (72.8575, 137.1578),
                    time: 2.0489,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9047, -3.1950),
                    velocity: (-0.9513, -1.5369),
                    acceleration: (65.2090, 118.1874),
                    time: 2.0544,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9011, -3.2006),
                    velocity: (-0.6475, -1.0032),
                    acceleration: (54.6929, 96.0608),
                    time: 2.0600,
                    pressure: 1.0
                }
            ]
        ));

        time += delta_time;
        let res5 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (4.825, -3.2),
            time: time,
            pressure: 1.0,
        });

        assert!(res5.is_ok());
        assert!(compare_results(
            res5.unwrap(),
            vec![ModelerResult {
                pos: (4.9868, -3.0389),
                velocity: (-2.5540, -4.5431),
                acceleration: (-418.1093, -252.8115),
                time: 2.0133,
                pressure: 1.0,
            }]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (4.9636, -3.0687),
                    velocity: (-4.1801, -5.3627),
                    acceleration: (-292.6871, -147.5319),
                    time: 2.0189,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9370, -3.0985),
                    velocity: (-4.7757, -5.3670),
                    acceleration: (-107.2116, -0.7651),
                    time: 2.0244,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9109, -3.1256),
                    velocity: (-4.6989, -4.8816),
                    acceleration: (13.8210, 87.3644),
                    time: 2.0300,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8875, -3.1486),
                    velocity: (-4.2257, -4.1466),
                    acceleration: (85.1835, 132.2997),
                    time: 2.0356,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8677, -3.1671),
                    velocity: (-3.5576, -3.3287),
                    acceleration: (120.2579, 147.2335),
                    time: 2.0411,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8520, -3.1812),
                    velocity: (-2.8333, -2.5353),
                    acceleration: (130.3700, 142.8088),
                    time: 2.0467,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8401, -3.1914),
                    velocity: (-2.1411, -1.8288),
                    acceleration: (124.5846, 127.1714),
                    time: 2.0522,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8316, -3.1982),
                    velocity: (-1.5312, -1.2386),
                    acceleration: (109.7874, 106.2279),
                    time: 2.0578,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8280, -3.2010),
                    velocity: (-1.2786, -1.0053),
                    acceleration: (90.9288, 84.0051),
                    time: 2.0606,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8272, -3.2017),
                    velocity: (-1.2209, -0.9529),
                    acceleration: (83.2052, 75.4288),
                    time: 2.0613,
                    pressure: 1.0
                }
            ]
        ));

        time += delta_time;
        let res6 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (4.75, -3.225),
            time: time,
            pressure: 1.0,
        });
        assert!(res6.is_ok());
        assert!(compare_results(
            res6.unwrap(),
            vec![ModelerResult {
                pos: (4.9726, -3.0565),
                velocity: (-4.2660, -5.2803),
                acceleration: (-513.5957, -221.1678),
                time: 2.0167,
                pressure: 1.0
            }]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (4.9381, -3.0894),
                    velocity: (-6.2018, -5.9261),
                    acceleration: (-348.4476, -116.2445),
                    time: 2.0222,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.9004, -3.1215),
                    velocity: (-6.7995, -5.7749),
                    acceleration: (-107.5834, 27.2264),
                    time: 2.0278,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8640, -3.1501),
                    velocity: (-6.5400, -5.1591),
                    acceleration: (46.7146, 110.8336),
                    time: 2.0333,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8319, -3.1741),
                    velocity: (-5.7897, -4.3207),
                    acceleration: (135.0462, 150.9226),
                    time: 2.0389,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8051, -3.1932),
                    velocity: (-4.8132, -3.4248),
                    acceleration: (175.7684, 161.2555),
                    time: 2.0444,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7841, -3.2075),
                    velocity: (-3.7898, -2.5759),
                    acceleration: (184.2227, 152.7958),
                    time: 2.0500,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7683, -3.2176),
                    velocity: (-2.8312, -1.8324),
                    acceleration: (172.5480, 133.8294),
                    time: 2.0556,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7572, -3.2244),
                    velocity: (-1.9986, -1.2198),
                    acceleration: (149.8577, 110.2830),
                    time: 2.0611,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7526, -3.2271),
                    velocity: (-1.6580, -0.9805),
                    acceleration: (122.6198, 86.1299),
                    time: 2.0639,
                    pressure: 1.0
                }
            ]
        ));

        time += delta_time;
        let res7 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (4.7, -3.3),
            time: time,
            pressure: 1.0,
        });
        assert!(res7.is_ok());
        assert!(compare_results(
            res7.unwrap(),
            vec![ModelerResult {
                pos: (4.9529, -3.0778),
                velocity: (-5.9184, -6.4042),
                acceleration: (-495.7209, -337.1538),
                time: 2.0200,
                pressure: 1.0
            }]
        ));
        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (4.9101, -3.1194),
                    velocity: (-7.6886, -7.4784),
                    acceleration: (-318.6394, -193.3594),
                    time: 2.0256,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8654, -3.1607),
                    velocity: (-8.0518, -7.4431),
                    acceleration: (-65.3698, 6.3579),
                    time: 2.0311,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8235, -3.1982),
                    velocity: (-7.5377, -6.7452),
                    acceleration: (92.5345, 125.6104),
                    time: 2.0367,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7872, -3.2299),
                    velocity: (-6.5440, -5.7133),
                    acceleration: (178.8654, 185.7426),
                    time: 2.0422,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7574, -3.2553),
                    velocity: (-5.3529, -4.5748),
                    acceleration: (214.4027, 204.9362),
                    time: 2.0478,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7344, -3.2746),
                    velocity: (-4.1516, -3.4758),
                    acceleration: (216.2348, 197.8224),
                    time: 2.0533,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7174, -3.2885),
                    velocity: (-3.0534, -2.5004),
                    acceleration: (197.6767, 175.5702),
                    time: 2.0589,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7056, -3.2979),
                    velocity: (-2.1169, -1.6879),
                    acceleration: (168.5711, 146.2573),
                    time: 2.0644,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7030, -3.3000),
                    velocity: (-1.9283, -1.5276),
                    acceleration: (135.7820, 115.3739),
                    time: 2.0658,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7017, -3.3010),
                    velocity: (-1.8380, -1.4512),
                    acceleration: (130.0928, 110.0859),
                    time: 2.0665,
                    pressure: 1.0
                },
            ]
        ));

        time += delta_time;
        let res8 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (4.675, -3.4),
            time: time,
            pressure: 1.0,
        });
        assert!(res8.is_ok());
        assert!(compare_results(
            res8.unwrap(),
            vec![ModelerResult {
                pos: (4.9288, -3.1046),
                velocity: (-7.2260, -8.0305),
                acceleration: (-392.2747, -487.9053),
                time: 2.0233,
                pressure: 1.0
            },]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (4.8816, -3.1582),
                    velocity: (-8.4881, -9.6525),
                    acceleration: (-227.1831, -291.9628),
                    time: 2.0289,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8345, -3.2124),
                    velocity: (-8.4738, -9.7482),
                    acceleration: (2.5870, -17.2266),
                    time: 2.0344,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7918, -3.2619),
                    velocity: (-7.6948, -8.9195),
                    acceleration: (140.2131, 149.1810),
                    time: 2.0400,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7555, -3.3042),
                    velocity: (-6.5279, -7.6113),
                    acceleration: (210.0428, 235.4638),
                    time: 2.0456,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7264, -3.3383),
                    velocity: (-5.2343, -6.1345),
                    acceleration: (232.8451, 265.8274),
                    time: 2.0511,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7043, -3.3643),
                    velocity: (-3.9823, -4.6907),
                    acceleration: (225.3593, 259.8790),
                    time: 2.0567,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6884, -3.3832),
                    velocity: (-2.8691, -3.3980),
                    acceleration: (200.3802, 232.6849),
                    time: 2.0622,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6776, -3.3961),
                    velocity: (-1.9403, -2.3135),
                    acceleration: (167.1764, 195.2152),
                    time: 2.0678,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6752, -3.3990),
                    velocity: (-1.7569, -2.0983),
                    acceleration: (132.0560, 154.9868),
                    time: 2.0692,
                    pressure: 1.0
                },
            ]
        ));
        time += delta_time;

        let res9 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (4.675, -3.525),
            time: time,
            pressure: 1.0,
        });
        assert!(res9.is_ok());
        assert!(compare_results(
            res9.unwrap(),
            vec![ModelerResult {
                pos: (4.9022, -3.1387),
                velocity: (-7.9833, -10.2310),
                acceleration: (-227.1831, -660.1446),
                time: 2.0267,
                pressure: 1.0
            },]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (4.8549, -3.2079),
                    velocity: (-8.5070, -12.4602),
                    acceleration: (-94.2781, -401.2599),
                    time: 2.0322,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8102, -3.2783),
                    velocity: (-8.0479, -12.6650),
                    acceleration: (82.6390, -36.8616),
                    time: 2.0378,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7711, -3.3429),
                    velocity: (-7.0408, -11.6365),
                    acceleration: (181.2765, 185.1286),
                    time: 2.0433,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7389, -3.3983),
                    velocity: (-5.7965, -9.9616),
                    acceleration: (223.9801, 301.4933),
                    time: 2.0489,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7137, -3.4430),
                    velocity: (-4.5230, -8.0510),
                    acceleration: (229.2397, 343.9032),
                    time: 2.0544,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6951, -3.4773),
                    velocity: (-3.3477, -6.1727),
                    acceleration: (211.5554, 338.0856),
                    time: 2.0600,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6821, -3.5022),
                    velocity: (-2.3381, -4.4846),
                    acceleration: (181.7131, 303.8597),
                    time: 2.0656,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6737, -3.5192),
                    velocity: (-1.5199, -3.0641),
                    acceleration: (147.2879, 255.7003),
                    time: 2.0711,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6718, -3.5231),
                    velocity: (-1.3626, -2.7813),
                    acceleration: (113.2437, 203.5595),
                    time: 2.0725,
                    pressure: 1.0
                },
            ]
        ));

        time += delta_time;
        // we get more results at the end of the stroke (stroke end catch up)
        let res10 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Up,
            pos: (4.7, -3.6),
            time: time,
            pressure: 1.0,
        });
        assert!(res10.is_ok());
        assert!(compare_results(
            res10.unwrap(),
            vec![
                ModelerResult {
                    pos: (4.8753, -3.1797),
                    velocity: (-8.0521, -12.3049),
                    acceleration: (-20.6429, -622.1685),
                    time: 2.0300,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.8325, -3.2589),
                    velocity: (-7.7000, -14.2607),
                    acceleration: (63.3680, -352.0363),
                    time: 2.0356,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7948, -3.3375),
                    velocity: (-6.7888, -14.1377),
                    acceleration: (164.0215, 22.1350),
                    time: 2.0411,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7636, -3.4085),
                    velocity: (-5.6249, -12.7787),
                    acceleration: (209.5020, 244.6249),
                    time: 2.0467,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7390, -3.4685),
                    velocity: (-4.4152, -10.8015),
                    acceleration: (217.7452, 355.8801),
                    time: 2.0522,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7208, -3.5164),
                    velocity: (-3.2880, -8.6333),
                    acceleration: (202.8961, 390.2804),
                    time: 2.0578,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.7079, -3.5528),
                    velocity: (-2.3128, -6.5475),
                    acceleration: (175.5414, 375.4407),
                    time: 2.0633,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6995, -3.5789),
                    velocity: (-1.5174, -4.7008),
                    acceleration: (143.1705, 332.4062),
                    time: 2.0689,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6945, -3.5965),
                    velocity: (-0.9022, -3.1655),
                    acceleration: (110.7325, 276.3669),
                    time: 2.0744,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (4.6942, -3.5976),
                    velocity: (-0.8740, -3.0899),
                    acceleration: (81.2036, 217.6189),
                    time: 2.0748,
                    pressure: 1.0
                },
            ]
        ));

        // the stroke is finished so there's nothing left to predict
        assert!(engine.predict().is_err());
    }

    #[test]
    fn wobble_smoothed() {
        let delta_time = 0.0167;
        let mut engine = StrokeModeler::default();

        let mut time = 4.0;
        let res1 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (-6.0, -2.0),
            time: time,
            pressure: 1.0,
        });
        assert!(res1.is_ok());
        assert!(compare_results(
            res1.unwrap(),
            vec![ModelerResult {
                pos: (-6.0, -2.0),
                time: 4.0,
                ..ModelerResult::default()
            }]
        ));

        time += delta_time;

        let res2 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (-6.02, -2.0),
            time: time,
            ..ModelerInput::default()
        });
        assert!(res2.is_ok());
        assert!(compare_results(
            res2.unwrap(),
            vec![
                ModelerResult {
                    pos: (-6.0003, -2.0),
                    velocity: (-0.0615, 0.0),
                    acceleration: (-14.7276, 0.0),
                    time: 4.0042,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0009, -2.0),
                    velocity: (-0.1628, 0.0),
                    acceleration: (-24.2725, 0.0),
                    time: 4.0084,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0021, -2.0),
                    velocity: (-0.2868, 0.0),
                    acceleration: (-29.6996, 0.0),
                    time: 4.0125,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0039, -2.0),
                    velocity: (-0.4203, 0.0),
                    acceleration: (-31.9728, 0.0),
                    time: 4.0167,
                    pressure: 1.0
                },
            ]
        ));

        time += delta_time;
        let res3 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (-6.02, -2.02),
            time: time,
            pressure: 1.0,
        });
        assert!(res3.is_ok());
        assert!(compare_results(
            res3.unwrap(),
            vec![
                ModelerResult {
                    pos: (-6.0059, -2.0001),
                    velocity: (-0.4921, -0.0307),
                    acceleration: (-17.1932, -7.3638),
                    time: 4.0209,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0081, -2.0005),
                    velocity: (-0.5170, -0.0814),
                    acceleration: (-5.9729, -12.1355),
                    time: 4.0251,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0102, -2.0010),
                    velocity: (-0.5079, -0.1434),
                    acceleration: (2.1807, -14.8493),
                    time: 4.0292,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0122, -2.0019),
                    velocity: (-0.4755, -0.2101),
                    acceleration: (7.7710, -15.9860),
                    time: 4.0334,
                    pressure: 1.0
                },
            ]
        ));
        time += delta_time;
        let res4 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (-6.04, -2.02),
            time: time,
            pressure: 1.0,
        });
        assert!(res4.is_ok());
        assert!(compare_results(
            res4.unwrap(),
            vec![
                ModelerResult {
                    pos: (-6.0141, -2.0030),
                    velocity: (-0.4489, -0.2563),
                    acceleration: (6.3733, -11.0507),
                    time: 4.0376,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0159, -2.0042),
                    velocity: (-0.4277, -0.2856),
                    acceleration: (5.0670, -7.0315),
                    time: 4.0418,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0176, -2.0055),
                    velocity: (-0.4115, -0.3018),
                    acceleration: (3.8950, -3.8603),
                    time: 4.0459,
                    pressure: 1.0
                },
                ModelerResult {
                    pos: (-6.0193, -2.0067),
                    velocity: (-0.3994, -0.3078),
                    acceleration: (2.8758, -1.4435),
                    time: 4.0501,
                    pressure: 1.0
                },
            ]
        ));

        time += delta_time;
        let res5 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (-6.04, -2.04),
            time: time,
            pressure: 1.0,
        });
        assert!(res5.is_ok());
        assert!(
            (compare_results(
                res5.unwrap(),
                vec![
                    ModelerResult {
                        pos: (-6.0209, -2.0082),
                        velocity: (-0.3910, -0.3372),
                        acceleration: (2.0142, -7.0427),
                        time: 4.0543,
                        pressure: 1.0
                    },
                    ModelerResult {
                        pos: (-6.0225, -2.0098),
                        velocity: (-0.3856, -0.3814),
                        acceleration: (1.3090, -10.5977),
                        time: 4.0585,
                        pressure: 1.0
                    },
                    ModelerResult {
                        pos: (-6.0241, -2.0116),
                        velocity: (-0.3825, -0.4338),
                        acceleration: (0.7470, -12.5399),
                        time: 4.0626,
                        pressure: 1.0
                    },
                    ModelerResult {
                        pos: (-6.0257, -2.0136),
                        velocity: (-0.3811, -0.4891),
                        acceleration: (0.3174, -13.2543),
                        time: 4.0668,
                        pressure: 1.0
                    },
                ]
            ))
        );
    }

    #[test]
    fn reset_stroke() {
        let mut engine = StrokeModeler::default();
        let delta_time = 1. / 50.;
        let mut time = 0.0;
        let res = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (-8.0, -10.0),
            time: time,
            pressure: 1.0,
        });
        assert!(res.is_ok());
        assert!(!res.unwrap().is_empty());
        assert!(engine.predict().is_ok());
        assert!(engine.predict().unwrap().is_empty());

        time += delta_time;
        let res2 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            time: time,
            ..ModelerInput::default()
        });
        assert!(res2.is_ok());
        assert!(!res2.unwrap().is_empty());
        assert!(engine.predict().is_ok());
        assert!(!engine.predict().unwrap().is_empty());

        time += delta_time;
        let res3 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            time: time,
            pos: (-11.0, -5.0),
            ..ModelerInput::default()
        });
        assert!(res3.is_ok());
        assert!(!res3.unwrap().is_empty());
        assert!(engine.predict().is_ok());
        assert!(!engine.predict().unwrap().is_empty());

        engine.reset();
        assert!(engine.predict().is_err());
    }

    #[test]
    fn ignore_input_before_down() {
        let mut engine = StrokeModeler::default();

        assert!(engine
            .update(ModelerInput {
                event_type: ModelerInputEventType::Move,
                ..ModelerInput::default()
            })
            .is_err());
        assert!(engine
            .update(ModelerInput {
                event_type: ModelerInputEventType::Up,
                ..ModelerInput::default()
            })
            .is_err());
    }

    #[test]
    fn tdown_in_progress_error() {
        let mut engine = StrokeModeler::default();

        assert!(engine
            .update(ModelerInput {
                event_type: ModelerInputEventType::Down,
                ..ModelerInput::default()
            })
            .is_ok());
        assert!(engine
            .update(ModelerInput {
                event_type: ModelerInputEventType::Down,
                ..ModelerInput::default()
            })
            .is_err());
    }

    #[test]
    fn alternate_params() {
        let delta_time = 1. / 50.;
        let mut engine = StrokeModeler::new(ModelerParams {
            sampling_min_output_rate: 70.0,
            stylus_state_modeler_max_input_samples: 20,
            ..ModelerParams::suggested()
        })
        .unwrap();

        let mut time = 3.0;

        let res1 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (0.0, 0.0),
            time: time,
            pressure: 0.5,
        });
        assert!(res1.is_ok());
        assert!(compare_results(
            res1.unwrap(),
            vec![ModelerResult {
                time: 3.0,
                pressure: 0.5,
                ..ModelerResult::default()
            }]
        ));

        assert!(engine.predict().is_ok());
        assert!(engine.predict().unwrap().is_empty());

        time += delta_time;

        let res2 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (0.0, 0.5),
            time: time,
            pressure: 0.4,
        });
        assert!(res2.is_ok());
        assert!(compare_results(
            res2.unwrap(),
            vec![
                ModelerResult {
                    pos: (0.0, 0.0736),
                    velocity: (0.0, 7.3636),
                    acceleration: (0.0, 736.3636),
                    time: 3.0100,
                    pressure: 0.4853
                },
                ModelerResult {
                    pos: (0.0, 0.2198),
                    velocity: (0.0, 14.6202),
                    acceleration: (0.0, 725.6529),
                    time: 3.0200,
                    pressure: 0.4560
                },
            ]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (0.0, 0.3823),
                    velocity: (0.0, 11.3709),
                    acceleration: (0.0, -227.4474),
                    time: 3.0343,
                    pressure: 0.4235
                },
                ModelerResult {
                    pos: (0.0, 0.4484),
                    velocity: (0.0, 4.6285),
                    acceleration: (0.0, -471.9660),
                    time: 3.0486,
                    pressure: 0.4103
                },
                ModelerResult {
                    pos: (0.0, 0.4775),
                    velocity: (0.0, 2.0389),
                    acceleration: (0.0, -181.2747),
                    time: 3.0629,
                    pressure: 0.4045
                },
                ModelerResult {
                    pos: (0.0, 0.4902),
                    velocity: (0.0, 0.8873),
                    acceleration: (0.0, -80.6136),
                    time: 3.0771,
                    pressure: 0.4020
                },
                ModelerResult {
                    pos: (0.0, 0.4957),
                    velocity: (0.0, 0.3868),
                    acceleration: (0.0, -35.0318),
                    time: 3.0914,
                    pressure: 0.4009
                },
                ModelerResult {
                    pos: (0.0, 0.4981),
                    velocity: (0.0, 0.1686),
                    acceleration: (0.0, -15.2760),
                    time: 3.1057,
                    pressure: 0.4004
                },
                ModelerResult {
                    pos: (0.0, 0.4992),
                    velocity: (0.0, 0.0735),
                    acceleration: (0.0, -6.6579),
                    time: 3.1200,
                    pressure: 0.4002
                },
            ]
        ));
        time += delta_time;

        let res3 = engine.update(ModelerInput {
            pos: (0.2, 1.0),
            time: time,
            pressure: 0.3,
            event_type: ModelerInputEventType::Move,
        });
        assert!(res3.is_ok());
        assert!(compare_results(
            res3.unwrap(),
            vec![
                ModelerResult {
                    pos: (0.0295, 0.4169),
                    velocity: (2.9455, 19.7093),
                    acceleration: (294.5455, 508.9161),
                    time: 3.0300,
                    pressure: 0.4166
                },
                ModelerResult {
                    pos: (0.0879, 0.6439),
                    velocity: (5.8481, 22.6926),
                    acceleration: (290.2612, 298.3311),
                    time: 3.0400,
                    pressure: 0.3691
                },
            ]
        ));

        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (0.1529, 0.8487),
                    velocity: (4.5484, 14.3374),
                    acceleration: (-90.9790, -584.8687),
                    time: 3.0543,
                    pressure: 0.3293
                },
                ModelerResult {
                    pos: (0.1794, 0.9338),
                    velocity: (1.8514, 5.9577),
                    acceleration: (-188.7864, -586.5760),
                    time: 3.0686,
                    pressure: 0.3128
                },
                ModelerResult {
                    pos: (0.1910, 0.9712),
                    velocity: (0.8156, 2.6159),
                    acceleration: (-72.5099, -233.9289),
                    time: 3.0829,
                    pressure: 0.3056
                },
                ModelerResult {
                    pos: (0.1961, 0.9874),
                    velocity: (0.3549, 1.1389),
                    acceleration: (-32.2455, -103.3868),
                    time: 3.0971,
                    pressure: 0.3024
                },
                ModelerResult {
                    pos: (0.1983, 0.9945),
                    velocity: (0.1547, 0.4965),
                    acceleration: (-14.0127, -44.9693),
                    time: 3.1114,
                    pressure: 0.3011
                },
                ModelerResult {
                    pos: (0.1993, 0.9976),
                    velocity: (0.0674, 0.2164),
                    acceleration: (-6.1104, -19.6068),
                    time: 3.1257,
                    pressure: 0.3005
                },
                ModelerResult {
                    pos: (0.1997, 0.9990),
                    velocity: (0.0294, 0.0943),
                    acceleration: (-2.6631, -8.5455),
                    time: 3.1400,
                    pressure: 0.3002
                },
            ]
        ));

        time += delta_time;
        let res4 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (0.4, 1.4),
            time: time,
            pressure: 0.2,
        });
        assert!(res4.is_ok());
        assert!(compare_results(
            res4.unwrap(),
            vec![
                ModelerResult {
                    pos: (0.1668, 0.8712),
                    velocity: (7.8837, 22.7349),
                    acceleration: (203.5665, 4.2224),
                    time: 3.0500,
                    pressure: 0.3245
                },
                ModelerResult {
                    pos: (0.2575, 1.0906),
                    velocity: (9.0771, 21.9411),
                    acceleration: (119.3324, -79.3721),
                    time: 3.0600,
                    pressure: 0.2761
                },
            ]
        ));
        assert!(engine.predict().is_ok());
        assert!(compare_results(
            engine.predict().unwrap(),
            vec![
                ModelerResult {
                    pos: (0.3395, 1.2676),
                    velocity: (5.7349, 12.3913),
                    acceleration: (-233.9475, -668.4906),
                    time: 3.0743,
                    pressure: 0.2325
                },
                ModelerResult {
                    pos: (0.3735, 1.3421),
                    velocity: (2.3831, 5.2156),
                    acceleration: (-234.6304, -502.2992),
                    time: 3.0886,
                    pressure: 0.2142
                },
                ModelerResult {
                    pos: (0.3885, 1.3748),
                    velocity: (1.0463, 2.2854),
                    acceleration: (-93.5716, -205.1091),
                    time: 3.1029,
                    pressure: 0.2062
                },
                ModelerResult {
                    pos: (0.3950, 1.3890),
                    velocity: (0.4556, 0.9954),
                    acceleration: (-41.3547, -90.3064),
                    time: 3.1171,
                    pressure: 0.2027
                },
                ModelerResult {
                    pos: (0.3978, 1.3952),
                    velocity: (0.1986, 0.4339),
                    acceleration: (-17.9877, -39.3021),
                    time: 3.1314,
                    pressure: 0.2012
                },
                ModelerResult {
                    pos: (0.3990, 1.3979),
                    velocity: (0.0866, 0.1891),
                    acceleration: (-7.8428, -17.1346),
                    time: 3.1457,
                    pressure: 0.2005
                },
                ModelerResult {
                    pos: (0.3996, 1.3991),
                    velocity: (0.0377, 0.0824),
                    acceleration: (-3.4182, -7.4680),
                    time: 3.1600,
                    pressure: 0.2002
                },
            ]
        ));
        time += delta_time;
        let res5 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Up,
            pos: (0.7, 1.7),
            pressure: 0.1,
            time: time,
        });
        assert!(res5.is_ok());
        assert!(compare_results(
            res5.unwrap(),
            vec![
                ModelerResult {
                    pos: (0.3691, 1.2874),
                    velocity: (11.1558, 19.6744),
                    acceleration: (207.8707, -226.6725),
                    time: 3.0700,
                    pressure: 0.2256
                },
                ModelerResult {
                    pos: (0.4978, 1.4640),
                    velocity: (12.8701, 17.6629),
                    acceleration: (171.4340, -201.1508),
                    time: 3.0800,
                    pressure: 0.1730
                },
                ModelerResult {
                    pos: (0.6141, 1.5986),
                    velocity: (8.1404, 9.4261),
                    acceleration: (-331.0815, -576.5752),
                    time: 3.0943,
                    pressure: 0.1312
                },
                ModelerResult {
                    pos: (0.6624, 1.6557),
                    velocity: (3.3822, 3.9953),
                    acceleration: (-333.0701, -380.1579),
                    time: 3.1086,
                    pressure: 0.1136
                },
                ModelerResult {
                    pos: (0.6836, 1.6807),
                    velocity: (1.4851, 1.7488),
                    acceleration: (-132.8005, -157.2520),
                    time: 3.1229,
                    pressure: 0.1059
                },
                ModelerResult {
                    pos: (0.6929, 1.6916),
                    velocity: (0.6466, 0.7618),
                    acceleration: (-58.6943, -69.0946),
                    time: 3.1371,
                    pressure: 0.1026
                },
                ModelerResult {
                    pos: (0.6969, 1.6963),
                    velocity: (0.2819, 0.3321),
                    acceleration: (-25.5298, -30.0794),
                    time: 3.1514,
                    pressure: 0.1011
                },
                ModelerResult {
                    pos: (0.6986, 1.6984),
                    velocity: (0.1229, 0.1447),
                    acceleration: (-11.1311, -13.1133),
                    time: 3.1657,
                    pressure: 0.1005
                },
                ModelerResult {
                    pos: (0.6994, 1.6993),
                    velocity: (0.0535, 0.0631),
                    acceleration: (-4.8514, -5.7153),
                    time: 3.1800,
                    pressure: 0.1002
                },
            ]
        ));
        assert!(engine.predict().is_err());
    }

    #[test]
    fn generate_output_up_nodelta() {
        let delta_time = 1. / 500.;
        let mut engine = StrokeModeler::default();
        let mut time = 0.0;

        let res1 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Down,
            pos: (5.0, 5.0),
            time: time,
            pressure: 1.0,
        });
        assert!(res1.is_ok());
        assert!(compare_results(
            res1.unwrap(),
            vec![ModelerResult {
                pos: (5.0, 5.0),
                time: 0.0,
                ..ModelerResult::default()
            }]
        ));
        time += delta_time;
        let res2 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Move,
            pos: (5.0, 5.0),
            pressure: 1.0,
            time: time,
        });
        assert!(res2.is_ok());
        assert!(compare_results(
            res2.unwrap(),
            vec![ModelerResult {
                pos: (5.0, 5.0),
                time: 0.002,
                ..ModelerResult::default()
            }]
        ));
        let res3 = engine.update(ModelerInput {
            event_type: ModelerInputEventType::Up,
            pos: (5.0, 5.0),
            time: time,
            pressure: 1.0,
        });
        assert!(res3.is_ok());
        assert!(compare_results(
            res3.unwrap(),
            vec![ModelerResult {
                pos: (5.0, 5.0),
                time: 0.0076,
                pressure: 1.0,
                ..ModelerResult::default()
            }]
        ));
    }

    // needed ? we already catch this in the rust code
    // #[test]
    // fn far_apart_times_move() {
    //     let mut engine = StrokeModeler::default();
    //     let res1=engine.update(ModelerInput {
    //         event_type: ModelerInputEventType::kDown,
    //         pos: (0.0,0.0),
    //         time:0.0,
    //         pressure:0.2
    //     });
    //     assert!(res1.is_ok());
    //     assert!(!res1.unwrap().is_empty());

    //     let res2 = engine.update(ModelerInput {
    //         event_type:ModelerInputEventType::Up,
    //         pos:(0.0,0.0),
    //         pressure:0.2,
    //         time: 2147483647.0,
    //     });
    //     assert!(res2.is_ok());

    // }
}
