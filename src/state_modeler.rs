use crate::position_modeler::dist;
use crate::utils::{interp, interp2, nearest_point_on_segment};
use crate::ModelerInput;
use std::collections::VecDeque;

/// Get the pressure for a position by querying
/// information from the raw input strokes
pub struct StateModeler {
    /// max number of elements
    stylus_state_modeler_max_input_samples: usize,
    /// deque holding the data from strokes
    last_strokes: VecDeque<ModelerInput>,
}

impl Default for StateModeler {
    fn default() -> Self {
        Self {
            stylus_state_modeler_max_input_samples: 10,
            last_strokes: VecDeque::new(),
        }
    }
}

impl StateModeler {
    pub fn new(param: usize) -> Self {
        Self {
            stylus_state_modeler_max_input_samples: param,
            last_strokes: VecDeque::new(),
        }
    }
    pub fn update(&mut self, input: ModelerInput) {
        // add the event to the strokes
        self.last_strokes.push_back(input);
        if self.last_strokes.len() > self.stylus_state_modeler_max_input_samples {
            self.last_strokes.pop_front();
        }
    }

    pub fn reset(&mut self, max_input: usize) {
        self.last_strokes = VecDeque::new();
        self.stylus_state_modeler_max_input_samples = max_input;
    }

    pub fn query(&mut self, pos: (f32, f32)) -> f32 {
        // iterate over the decque
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
    assert_eq!(state_mod.query((0.0, 0.0)), 1.0); // 1 is our "unknown" default value
    assert_eq!(state_mod.query((-5.0, 3.0)), 1.0); // 1 is our "unknown" default value
}
#[test]
fn query_single_output() {
    let mut state_mod = StateModeler::new(10);
    state_mod.update(ModelerInput {
        pos: (0.0, 0.0),
        pressure: 0.75,
        ..ModelerInput::default()
    });

    assert_eq!(state_mod.query((0.0, 0.0)), 0.75);
    assert_eq!(state_mod.query((1.0, 1.0)), 0.75);
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
    assert!((state_mod.query((0.0, 2.0)) - 0.3).abs() < tol);
    assert!((state_mod.query((1.0, 2.0)) - 0.4).abs() < tol);
    assert!((state_mod.query((2.0, 1.5)) - 0.6).abs() < tol);
    assert!((state_mod.query((2.5, 1.875)) - 0.65).abs() < tol);
    assert!((state_mod.query((2.5, 3.125)) - 0.75).abs() < tol);
    assert!((state_mod.query((2.5, 4.0)) - 0.8).abs() < tol);
    assert!((state_mod.query((3.0, 4.0)) - 0.5).abs() < tol);
    assert!((state_mod.query((4.0, 4.0)) - 0.2).abs() < tol);
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
    assert!((state_mod.query((2.0, 0.0)) - 0.6).abs() < tol);
    assert!((state_mod.query((1.0, 3.5)) - 0.45).abs() < tol);
    assert!((state_mod.query((-3.0, 17. / 6.)) - 0.5).abs() < tol);

    //adds a 11-th point so that the first point is discarded
    state_mod.update(ModelerInput {
        pos: (-8.0, 0.0),
        pressure: 0.6,
        ..Default::default()
    });

    assert!((state_mod.query((2.0, 0.0)) - 0.3).abs() < tol);
    assert!((state_mod.query((1.0, 3.5)) - 0.3).abs() < tol);
    assert!((state_mod.query((-3.0, 17. / 6.)) - 0.5).abs() < tol);

    state_mod.update(ModelerInput {
        pos: (-8.0, 0.0),
        pressure: 0.6,
        ..Default::default()
    });

    assert!((state_mod.query((2.0, 0.0)) - 0.9).abs() < tol);
    assert!((state_mod.query((1.0, 3.5)) - 0.9).abs() < tol);
    assert!((state_mod.query((-3.0, 17. / 6.)) - 0.9).abs() < tol);
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
    assert!((state_mod.query((10.0, 12.0)) - 0.1).abs() < tol);
    state_mod.reset(10);

    assert_eq!(state_mod.query((10.0, 12.0)), 1.0); //unknown

    state_mod.update(ModelerInput {
        pos: (-1.0, 4.0),
        pressure: 0.4,
        ..Default::default()
    });

    assert!((state_mod.query((6.0, 7.0)) - 0.4).abs() < tol);

    state_mod.update(ModelerInput {
        pos: (-3.0, 0.0),
        pressure: 0.7,
        ..Default::default()
    });

    assert!((state_mod.query((-2.0, 2.0)) - 0.55).abs() < tol);
    assert!((state_mod.query((0.0, 5.0)) - 0.4).abs() < tol);
}

// remark : we suppose that pressure is always defined
// and is set to 1 otherwise (both for input and outputs)
