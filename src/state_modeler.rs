use crate::utils::{dist, interp, interp2, nearest_point_on_segment};
use crate::ModelerInput;

// only imported for docstrings
#[allow(unused)]
use crate::ModelerPartial;
#[allow(unused)]
use crate::ModelerResult;

use std::collections::VecDeque;

/// Get the pressure for a position by querying
/// information from the raw input strokes
///
/// All raw input strokes are to be provided to this state modeler by calling `update`
/// Then [ModelerPartial] structs can be converted to [ModelerResult] by querying the
/// pressure data by calling this struct with the `query` function
#[doc = include_str!("../docs/notations.html")]
#[doc = include_str!("../docs/stylus_state_modeler.html")]
pub(crate) struct StateModeler {
    /// max number of elements
    stylus_state_modeler_max_input_samples: usize,
    /// deque holding the data from strokes
    last_strokes: VecDeque<ModelerInput>,
}

impl Default for StateModeler {
    fn default() -> Self {
        Self {
            stylus_state_modeler_max_input_samples: 10,
            last_strokes: VecDeque::with_capacity(11),
        }
    }
}

impl StateModeler {
    /// initialize a new StateModeler
    pub(crate) fn new(param: usize) -> Self {
        Self {
            stylus_state_modeler_max_input_samples: param,
            last_strokes: VecDeque::with_capacity(param + 1),
        }
    }

    /// add the most recent raw input to the StateModeler
    pub(crate) fn update(&mut self, input: ModelerInput) {
        // add the event to the strokes
        self.last_strokes.push_back(input);
        if self.last_strokes.len() > self.stylus_state_modeler_max_input_samples {
            self.last_strokes.pop_front();
        }
    }

    /// reset the StateModeler
    pub(crate) fn reset(&mut self, max_input: usize) {
        self.last_strokes = VecDeque::new();
        self.stylus_state_modeler_max_input_samples = max_input;
    }

    /// query the pressure by interpolating it from raw input events
    pub(crate) fn query(&mut self, pos: (f32, f32)) -> f32 {
        // iterate over the deque
        match self.last_strokes.len() {
            0 => 1.0,
            1 => return self.last_strokes.front().unwrap().pressure,
            _ => {
                let mut distance = f32::INFINITY;
                let mut r: f32 = 0.0;

                let mut start_pressure: f32 = 1.0;
                let mut end_pressure: f32 = 1.0;

                for index_it in 0..self.last_strokes.len() - 1 {
                    let start_pos = self.last_strokes.get(index_it).unwrap().pos;
                    let end_pos = self.last_strokes.get(index_it + 1).unwrap().pos;

                    let r_c = nearest_point_on_segment(start_pos, end_pos, pos);
                    let point_c = interp2(start_pos, end_pos, r_c);

                    if dist(pos, point_c) < distance {
                        distance = dist(pos, point_c);
                        r = r_c;
                        start_pressure = self.last_strokes.get(index_it).unwrap().pressure;
                        end_pressure = self.last_strokes.get(index_it + 1).unwrap().pressure;
                    }
                }

                interp(start_pressure, end_pressure, r)
            }
        }
    }
}

#[test]
fn state_modeler_straight() {
    let mut state_mod = StateModeler::new(10);
    approx::assert_relative_eq!(state_mod.query((0.0, 0.0)), 1.0); // 1 is our "unknown" default value
    approx::assert_relative_eq!(state_mod.query((-5.0, 3.0)), 1.0); // 1 is our "unknown" default value
}
#[test]
fn query_single_output() {
    let mut state_mod = StateModeler::new(10);
    state_mod.update(ModelerInput {
        pos: (0.0, 0.0),
        pressure: 0.75,
        ..ModelerInput::default()
    });

    approx::assert_relative_eq!(state_mod.query((0.0, 0.0)), 0.75);
    approx::assert_relative_eq!(state_mod.query((1.0, 1.0)), 0.75);
}

#[test]
fn query_multiple_output() {
    let mut state_mod = StateModeler::default();
    state_mod.update(ModelerInput {
        pos: (0.5, 1.5),
        pressure: 0.3,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (2.0, 1.5),
        pressure: 0.6,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (3.0, 3.5),
        pressure: 0.8,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (3.5, 4.0),
        pressure: 0.2,
        ..Default::default()
    });

    let tol = 1e-5;
    approx::assert_abs_diff_eq!(state_mod.query((0.0, 2.0)), 0.3, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((1.0, 2.0)), 0.4, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((2.0, 1.5)), 0.6, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((2.5, 1.875)), 0.65, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((2.5, 3.125)), 0.75, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((2.5, 4.0)), 0.8, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((2.5, 4.0)), 0.8, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((3.0, 4.0)), 0.5, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((4.0, 4.0)), 0.2, epsilon = tol);
}

#[test]
fn query_stale() {
    let mut state_mod = StateModeler::default();
    state_mod.update(ModelerInput {
        pos: (1.0, 1.0),
        pressure: 0.6,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-1.0, 2.0),
        pressure: 0.3,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-4.0, 0.0),
        pressure: 0.9,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-6.0, -3.0),
        pressure: 0.4,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-5.0, -5.0),
        pressure: 0.3,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-3.0, -4.0),
        pressure: 0.6,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-6.0, -7.0),
        pressure: 0.9,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-9.0, -8.0),
        pressure: 0.8,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-11.0, -5.0),
        pressure: 0.2,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (-10.0, -2.0),
        pressure: 0.7,
        ..Default::default()
    });

    let tol = 1e-5;
    approx::assert_abs_diff_eq!(state_mod.query((2.0, 0.0)), 0.6, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((1.0, 3.5)), 0.45, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((-3.0, 17. / 6.)), 0.5, epsilon = tol);

    //adds a 11-th point so that the first point is discarded
    state_mod.update(ModelerInput {
        pos: (-8.0, 0.0),
        pressure: 0.6,
        ..Default::default()
    });

    approx::assert_abs_diff_eq!(state_mod.query((2.0, 0.0)), 0.3, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((1.0, 3.5)), 0.3, epsilon = tol);
    approx::assert_relative_eq!(state_mod.query((-3.0, 17. / 6.)), 0.5, epsilon = tol);

    state_mod.update(ModelerInput {
        pos: (-8.0, 0.0),
        pressure: 0.6,
        ..Default::default()
    });

    approx::assert_abs_diff_eq!(state_mod.query((2.0, 0.0)), 0.9, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((1.0, 3.5)), 0.9, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((-3.0, 17. / 6.)), 0.9, epsilon = tol);
}

#[test]
fn query_reset() {
    let mut state_mod = StateModeler::default();
    state_mod.update(ModelerInput {
        pos: (4.0, 5.0),
        pressure: 0.4,
        ..Default::default()
    });
    state_mod.update(ModelerInput {
        pos: (7.0, 8.0),
        pressure: 0.1,
        ..Default::default()
    });

    let tol = 1e-5;
    approx::assert_abs_diff_eq!(state_mod.query((10.0, 12.0)), 0.1, epsilon = tol);
    state_mod.reset(10);

    approx::assert_relative_eq!(state_mod.query((10.0, 12.0)), 1.0);

    state_mod.update(ModelerInput {
        pos: (-1.0, 4.0),
        pressure: 0.4,
        ..Default::default()
    });

    approx::assert_abs_diff_eq!(state_mod.query((6.0, 7.0)), 0.4, epsilon = tol);

    state_mod.update(ModelerInput {
        pos: (-3.0, 0.0),
        pressure: 0.7,
        ..Default::default()
    });

    approx::assert_abs_diff_eq!(state_mod.query((-2.0, 2.0)), 0.55, epsilon = tol);
    approx::assert_abs_diff_eq!(state_mod.query((0.0, 5.0)), 0.4, epsilon = tol);
}
// remark : we suppose that pressure is always defined
// and is set to 1 otherwise (both for input and outputs)
