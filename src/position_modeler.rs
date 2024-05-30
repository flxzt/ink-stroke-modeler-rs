use crate::utils::{dist, nearest_point_on_segment};
use crate::{ModelerInput, ModelerParams, ModelerPartial};

/// This struct models the movement of the pen tip based on the laws of motion.
/// The pen tip is represented as a mass, connected by a spring to a moving
/// anchor; as the anchor moves, it drags the pen tip along behind it.
pub(crate) struct PositionModeler {
    //parameters for the model
    position_modeler_spring_mass_constant: f32,
    position_modeler_drag_constant: f32,
    // last state
    pub(crate) state: ModelerPartial,
}

impl PositionModeler {
    pub(crate) fn new(params: ModelerParams, first_input: ModelerInput) -> Self {
        Self {
            position_modeler_spring_mass_constant: params.position_modeler_spring_mass_constant,
            position_modeler_drag_constant: params.position_modeler_drag_constant,
            state: ModelerPartial {
                pos: first_input.pos,
                velocity: (0.0, 0.0),
                acceleration: (0.0, 0.0),
                time: first_input.time,
            },
        }
    }
    // Given the position of the anchor and the time, updates the model and
    // returns the new state of the pen tip
    pub(crate) fn update(&mut self, anchor_pos: (f32, f32), time: f64) -> ModelerPartial {
        let delta_time = (time - self.state.time) as f32;
        //
        self.state.acceleration = (
            (anchor_pos.0 - self.state.pos.0) / (self.position_modeler_spring_mass_constant)
                - self.position_modeler_drag_constant * self.state.velocity.0,
            (anchor_pos.1 - self.state.pos.1) / (self.position_modeler_spring_mass_constant)
                - self.position_modeler_drag_constant * self.state.velocity.1,
        );
        self.state.velocity = (
            self.state.velocity.0 + (delta_time) * self.state.acceleration.0,
            self.state.velocity.1 + (delta_time) * self.state.acceleration.1,
        );
        self.state.pos = (
            self.state.pos.0 + delta_time * self.state.velocity.0,
            self.state.pos.1 + delta_time * self.state.velocity.1,
        );
        self.state.time = time;

        self.state.clone()
    }

    /// update the model `n_steps` time between events
    /// this upsample between inputs linearly and applies
    /// these upstreamed events to the model
    pub(crate) fn update_along_linear_path(
        &mut self,
        start_pos: (f32, f32),
        start_time: f64,
        end_pos: (f32, f32),
        end_time: f64,
        n_steps: i32,
    ) -> Vec<ModelerPartial> {
        (1..=n_steps)
            .map(|i| {
                let frac_adv: f32 = i as f32 / n_steps as f32;

                let anchor_pos = (
                    start_pos.0 + frac_adv * (end_pos.0 - start_pos.0),
                    start_pos.1 + frac_adv * (end_pos.1 - start_pos.1),
                );
                let time = start_time + frac_adv as f64 * (end_time - start_time);

                self.update(anchor_pos, time)
            })
            .collect()
    }

    /// models the end of the stroke (catch-up) WITHOUT modifying the predictor
    /// (the state is saved then restored after calculations are done)
    ///
    /// This creates candidates solution using the latest event as an anchor
    /// but stops after `max_iterations`, if the distance between states is less
    /// than `stop_distance` or the candidate is close to the anchor (less than
    /// `stop_distance`)
    pub(crate) fn model_end_of_stroke(
        &mut self,
        anchor_pos: (f32, f32),
        delta_time: f64,
        max_iterations: usize,
        stop_distance: f32,
    ) -> Vec<ModelerPartial> {
        let initial_state = self.state.clone();
        let mut delta_time = delta_time;

        let mut out_events = Vec::<ModelerPartial>::with_capacity(max_iterations);
        for _ in 0..max_iterations {
            let previous_state = self.state.clone();
            let candidate = self.update(anchor_pos, previous_state.time + delta_time);

            if dist(previous_state.pos, candidate.pos) < stop_distance {
                // reset the state
                self.state = initial_state;
                // stop, we aren't making progress anymore
                return out_events;
            }

            if nearest_point_on_segment(
                (previous_state.pos.0, previous_state.pos.1),
                (candidate.pos.0, candidate.pos.1),
                (anchor_pos.0, anchor_pos.1),
            ) < 1.0
            {
                // overshoot, try with a smaller delta t
                delta_time *= 0.5;
                self.state = previous_state;
                continue;
            } else {
                out_events.push(candidate.clone());
            }

            if dist(candidate.pos, anchor_pos) < stop_distance {
                // very close to the anchor, stopping iterations
                // reset the state
                self.state = initial_state;
                return out_events;
            }
        }
        self.state = initial_state;
        out_events
    }
}

impl ModelerPartial {
    #[cfg(test)]
    fn near(self, compare: ModelerPartial) -> bool {
        let tol = 0.0005; //same tol as the ones used in the original repository
        approx::abs_diff_eq!(self.pos.0, compare.pos.0, epsilon = tol)
            && approx::abs_diff_eq!(self.pos.1, compare.pos.1, epsilon = tol)
            && approx::abs_diff_eq!(self.velocity.0, compare.velocity.0, epsilon = tol)
            && approx::abs_diff_eq!(self.velocity.1, compare.velocity.1, epsilon = tol)
            && approx::abs_diff_eq!(self.acceleration.0, compare.acceleration.0, epsilon = tol)
            && approx::abs_diff_eq!(self.acceleration.1, compare.acceleration.1, epsilon = tol)
            && approx::abs_diff_eq!(self.time, compare.time, epsilon = tol as f64)
    }
}

#[test]
fn straight_line() {
    // init
    let mut modeler: PositionModeler =
        PositionModeler::new(ModelerParams::suggested(), ModelerInput::default());
    let default_ts = 1. / 180 as f64;
    let mut current_time: f64 = 0.0;

    current_time += default_ts;
    assert!(modeler
        .update((1.0, 0.0), current_time)
        .near(ModelerPartial {
            pos: (0.0909, 0.0),
            velocity: (16.3636, 0.0),
            acceleration: (2945.4546, 0.0),
            time: current_time
        }));

    current_time += default_ts;
    assert!(modeler
        .update((2.0, 0.0), current_time)
        .near(ModelerPartial {
            pos: (0.319, 0.0),
            velocity: (41.0579, 0.0),
            acceleration: (4444.9590, 0.0),
            time: current_time
        }));

    current_time += default_ts;
    assert!(modeler
        .update((3.0, 0.0), current_time)
        .near(ModelerPartial {
            pos: (0.6996, 0.0),
            velocity: (68.5055, 0.0),
            acceleration: (4940.5737, 0.0),
            time: current_time
        }));

    current_time += default_ts;
    assert!(modeler
        .update((4.0, 0.0), current_time)
        .near(ModelerPartial {
            pos: (1.228, 0.0),
            velocity: (95.1099, 0.0),
            acceleration: (4788.8003, 0.0),
            time: current_time
        }));
}

#[test]
fn zigzag() {
    // init
    let mut current_time: f64 = 3.0;
    let mut modeler = PositionModeler::new(
        ModelerParams::suggested(),
        ModelerInput {
            pos: (-1.0, -1.0),
            time: current_time,
            ..ModelerInput::default()
        },
    );
    let default_ts = 1. / 180 as f64;

    current_time += default_ts;
    assert!(modeler
        .update((-0.5, -1.0), current_time)
        .near(ModelerPartial {
            pos: (-0.9545, -1.0),
            velocity: (8.1818, 0.0),
            acceleration: (1472.7273, 0.0),
            time: current_time
        }));

    current_time += default_ts;
    assert!(modeler
        .update((-0.5, -0.5), current_time)
        .near(ModelerPartial {
            pos: (-0.886, -0.9545),
            velocity: (12.3471, 8.1818),
            acceleration: (749.7521, 1472.7273),
            time: current_time
        }));

    current_time += default_ts;
    assert!(modeler
        .update((-0.0, -0.5), current_time)
        .near(ModelerPartial {
            pos: (-0.7643, -0.886),
            velocity: (21.9056, 12.3471),
            acceleration: (1720.5348, 749.7521),
            time: current_time
        }));

    current_time += default_ts;
    assert!(modeler
        .update((0.0, 0.0), current_time)
        .near(ModelerPartial {
            pos: (-0.6218, -0.7643),
            velocity: (25.6493, 21.9056),
            acceleration: (673.8650, 1720.5348),
            time: current_time
        }));

    current_time += default_ts;
    assert!(modeler
        .update((0.5, 0.0), current_time)
        .near(ModelerPartial {
            pos: (-0.4343, -0.6218),
            velocity: (33.7456, 25.6493),
            acceleration: (1457.3298, 673.8650),
            time: current_time
        }))
}

#[test]
fn sharp_turn() {
    // init
    let mut current_time: f64 = 1.6;
    let mut modeler = PositionModeler::new(
        ModelerParams::suggested(),
        ModelerInput {
            time: current_time,
            ..ModelerInput::default()
        },
    );
    let default_ts = 1. / 180 as f64;

    current_time += default_ts;
    assert!(modeler
        .update((0.25, 0.25), current_time)
        .near(ModelerPartial {
            pos: (0.0227, 0.0227),
            velocity: (4.0909, 4.0909),
            acceleration: (736.3636, 736.3636),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update((0.5, 0.5), current_time)
        .near(ModelerPartial {
            pos: (0.0798, 0.0798),
            velocity: (10.2645, 10.2645),
            acceleration: (1111.2397, 1111.2397),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update((0.75, 0.75), current_time)
        .near(ModelerPartial {
            pos: (0.1749, 0.1749),
            velocity: (17.1264, 17.1264),
            acceleration: (1235.1434, 1235.1434),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update((1.0, 1.0), current_time)
        .near(ModelerPartial {
            pos: (0.307, 0.307),
            velocity: (23.7775, 23.7775),
            acceleration: (1197.2001, 1197.2001),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update((1.25, 0.75), current_time)
        .near(ModelerPartial {
            pos: (0.472, 0.4265),
            velocity: (29.6975, 21.5157),
            acceleration: (1065.5977, -407.1296),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update((1.5, 0.5), current_time)
        .near(ModelerPartial {
            pos: (0.6644, 0.5049),
            velocity: (34.6406, 14.1117),
            acceleration: (889.7637, -1332.7158),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update((1.75, 0.25), current_time)
        .near(ModelerPartial {
            pos: (0.8786, 0.5288),
            velocity: (38.5482, 4.2955),
            acceleration: (703.3755, -1766.9114),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update((2.0, 0.0), current_time)
        .near(ModelerPartial {
            pos: (1.109, 0.495),
            velocity: (41.4794, -6.0756),
            acceleration: (527.5996, -1866.8005),
            time: current_time
        }));
}

#[test]
fn smooth_turn() {
    use std::f32::consts::PI;
    let point_on_circle = |x: f32| (x.cos(), x.sin());
    // init
    let mut current_time: f64 = 1.0;
    let mut modeler = PositionModeler::new(
        ModelerParams::suggested(),
        ModelerInput {
            time: current_time,
            pos: point_on_circle(0.0),
            ..ModelerInput::default()
        },
    );
    let default_ts = 1. / 180 as f64;
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI * 0.125), current_time)
        .near(ModelerPartial {
            pos: (0.9931, 0.0348),
            velocity: (-1.2456, 6.2621),
            acceleration: (-224.2095, 1127.1768),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI * 0.25), current_time)
        .near(ModelerPartial {
            pos: (0.9629, 0.1168),
            velocity: (-5.4269, 14.7588),
            acceleration: (-752.6373, 1529.4097),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI * 0.375), current_time)
        .near(ModelerPartial {
            pos: (0.8921, 0.2394),
            velocity: (-12.7511, 22.0623),
            acceleration: (-1318.3523, 1314.6320),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI * 0.5), current_time)
        .near(ModelerPartial {
            pos: (0.7685, 0.3820),
            velocity: (-22.2485, 25.6844),
            acceleration: (-1709.5339, 651.9690),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI * 0.625), current_time)
        .near(ModelerPartial {
            pos: (0.5897, 0.5169),
            velocity: (-32.1865, 24.2771),
            acceleration: (-1788.8300, -253.3177),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI * 0.75), current_time)
        .near(ModelerPartial {
            pos: (0.3645, 0.6151),
            velocity: (-40.5319, 17.6785),
            acceleration: (-1502.1846, -1187.7462),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI * 0.875), current_time)
        .near(ModelerPartial {
            pos: (0.1123, 0.6529),
            velocity: (-45.4017, 6.8034),
            acceleration: (-876.5552, -1957.5056),
            time: current_time
        }));
    current_time += default_ts;
    assert!(modeler
        .update(point_on_circle(PI), current_time)
        .near(ModelerPartial {
            pos: (-0.1402, 0.6162),
            velocity: (-45.4417, -6.6022),
            acceleration: (-7.2061, -2413.0093),
            time: current_time
        }));
}

#[test]
fn test_update_linear_path() {
    let mut modeler = PositionModeler::new(
        ModelerParams::suggested(),
        ModelerInput {
            time: 3.0,
            pos: (5.0, 10.0),
            ..ModelerInput::default()
        },
    );

    let linear_path = modeler.update_along_linear_path((5.0, 10.0), 3.0, (15., 10.), 3.05, 5);
    let expected = vec![
        ModelerPartial {
            pos: (5.5891, 10.0),
            velocity: (58.9091, 0.0),
            acceleration: (5890.9092, 0.0),
            time: 3.01,
        },
        ModelerPartial {
            pos: (6.7587, 10.0),
            velocity: (116.9613, 0.0),
            acceleration: (5805.2231, 0.0),
            time: 3.02,
        },
        ModelerPartial {
            pos: (8.3355, 10.0),
            velocity: (157.6746, 0.0),
            acceleration: (4071.3291, 0.0),
            time: 3.03,
        },
        ModelerPartial {
            pos: (10.1509, 10.0),
            velocity: (181.5411, 0.0),
            acceleration: (2386.6475, 0.0),
            time: 3.04,
        },
        ModelerPartial {
            pos: (12.0875, 10.0),
            velocity: (193.6607, 0.0),
            acceleration: (1211.9609, 0.0),
            time: 3.05,
        },
    ];

    assert!(linear_path
        .into_iter()
        .zip(expected)
        .fold(true, |acc, x| { acc && x.0.near(x.1) }));

    // second try
    let linear_path_2 = modeler.update_along_linear_path((15.0, 10.0), 3.05, (15.0, 16.0), 3.08, 3);
    let expected2 = vec![
        ModelerPartial {
            pos: (13.4876, 10.5891),
            velocity: (140.0123, 58.9091),
            acceleration: (-5364.8398, 5890.9092),
            time: 3.06,
        },
        ModelerPartial {
            pos: (14.3251, 11.7587),
            velocity: (83.7508, 116.9613),
            acceleration: (-5626.1528, 5805.2217),
            time: 3.07,
        },
        ModelerPartial {
            pos: (14.7584, 13.3355),
            velocity: (43.3291, 157.6746),
            acceleration: (-4042.1616, 4071.3291),
            time: 3.08,
        },
    ];

    assert!(linear_path_2
        .into_iter()
        .zip(expected2)
        .fold(true, |acc, x| { acc && x.0.near(x.1) }));
}

#[test]
fn model_end_of_stroke_stationnary() {
    let mut model = PositionModeler::new(
        ModelerParams::suggested(),
        ModelerInput {
            pos: (4.0, -2.0),
            ..ModelerInput::default()
        },
    );

    let result = model.model_end_of_stroke((3.0, -1.0), 1. / 180., 20, 0.01);
    let expected = vec![
        ModelerPartial {
            pos: (3.9091, -1.9091),
            velocity: (-16.3636, 16.3636),
            acceleration: (-2945.4546, 2945.4546),
            time: 0.0056,
        },
        ModelerPartial {
            pos: (3.7719, -1.7719),
            velocity: (-24.6942, 24.6942),
            acceleration: (-1499.5044, 1499.5042),
            time: 0.0111,
        },
        ModelerPartial {
            pos: (3.6194, -1.6194),
            velocity: (-27.4476, 27.4476),
            acceleration: (-495.6155, 495.6150),
            time: 0.0167,
        },
        ModelerPartial {
            pos: (3.4716, -1.4716),
            velocity: (-26.6045, 26.6044),
            acceleration: (151.7738, -151.7742),
            time: 0.0222,
        },
        ModelerPartial {
            pos: (3.3401, -1.3401),
            velocity: (-23.6799, 23.6799),
            acceleration: (526.4102, -526.4102),
            time: 0.0278,
        },
        ModelerPartial {
            pos: (3.2302, -1.2302),
            velocity: (-19.7725, 19.7725),
            acceleration: (703.3362, -703.3359),
            time: 0.0333,
        },
        ModelerPartial {
            pos: (3.1434, -1.1434),
            velocity: (-15.6306, 15.6306),
            acceleration: (745.5521, -745.5518),
            time: 0.0389,
        },
        ModelerPartial {
            pos: (3.0782, -1.0782),
            velocity: (-11.7244, 11.7244),
            acceleration: (703.1044, -703.1039),
            time: 0.0444,
        },
        ModelerPartial {
            pos: (3.0320, -1.0320),
            velocity: (-8.3149, 8.3149),
            acceleration: (613.7169, -613.7166),
            time: 0.0500,
        },
        ModelerPartial {
            pos: (3.0014, -1.0014),
            velocity: (-5.5133, 5.5133),
            acceleration: (504.2921, -504.2918),
            time: 0.0556,
        },
    ];

    assert!(result
        .into_iter()
        .zip(expected)
        .fold(true, |acc, x| { acc && x.0.near(x.1) }));
}

#[test]
fn end_of_stroke_motion() {
    let mut model = PositionModeler {
        position_modeler_drag_constant: ModelerParams::suggested().position_modeler_drag_constant,
        position_modeler_spring_mass_constant: ModelerParams::suggested()
            .position_modeler_spring_mass_constant,
        state: ModelerPartial {
            pos: (-1.0, 2.0),
            velocity: (40.0, 10.0),
            acceleration: (0.0, 0.0),
            time: 1.,
        },
    };

    let result = model.model_end_of_stroke((7.0, 2.0), 1. / 120., 20, 0.01);
    let expected = vec![
        ModelerPartial {
            pos: (0.7697, 2.0333),
            velocity: (212.3636, 4.0000),
            acceleration: (20683.6367, -720.0000),
            time: 1.0083,
        },
        ModelerPartial {
            pos: (2.7520, 2.0398),
            velocity: (237.8711, 0.7818),
            acceleration: (3060.8916, -386.1817),
            time: 1.0167,
        },
        ModelerPartial {
            pos: (4.4138, 2.0343),
            velocity: (199.4186, -0.6654),
            acceleration: (-4614.2959, -173.6631),
            time: 1.0250,
        },
        ModelerPartial {
            pos: (5.6075, 2.0251),
            velocity: (143.2474, -1.1081),
            acceleration: (-6740.5410, -53.1330),
            time: 1.0333,
        },
        ModelerPartial {
            pos: (6.3698, 2.0162),
            velocity: (91.4784, -1.0586),
            acceleration: (-6212.2896, 5.9471),
            time: 1.0417,
        },
        ModelerPartial {
            pos: (6.8037, 2.0094),
            velocity: (52.0592, -0.8222),
            acceleration: (-4730.2935, 28.3621),
            time: 1.0500,
        },
        ModelerPartial {
            pos: (6.9655, 2.0065),
            velocity: (38.8512, -0.6909),
            acceleration: (-3169.9351, 31.5268),
            time: 1.0542,
        },
        ModelerPartial {
            pos: (6.9850, 2.0062),
            velocity: (37.4471, -0.6750),
            acceleration: (-2695.7649, 30.5478),
            time: 1.0547,
        },
    ];
    assert!(result
        .into_iter()
        .zip(expected)
        .fold(true, |acc, x| { acc && x.0.near(x.1) }));
}

#[test]
fn end_of_stroke_maxiters() {
    let mut model = PositionModeler {
        position_modeler_drag_constant: ModelerParams::suggested().position_modeler_drag_constant,
        position_modeler_spring_mass_constant: ModelerParams::suggested()
            .position_modeler_spring_mass_constant,
        state: ModelerPartial {
            pos: (8.0, -3.0),
            velocity: (-100.0, -150.0),
            acceleration: (0.0, 0.0),
            time: 1.,
        },
    };

    let result = model.model_end_of_stroke((-9., -10.0), 0.0001, 10, 0.001);
    let expected = vec![
        ModelerPartial {
            pos: (7.9896, -3.0151),
            velocity: (-104.2873, -150.9818),
            acceleration: (-42872.7266, -9818.1816),
            time: 1.0001,
        },
        ModelerPartial {
            pos: (7.9787, -3.0303),
            velocity: (-108.5406, -151.9521),
            acceleration: (-42533.3242, -9703.0205),
            time: 1.0002,
        },
        ModelerPartial {
            pos: (7.9674, -3.0456),
            velocity: (-112.7601, -152.9110),
            acceleration: (-42195.1211, -9588.4023),
            time: 1.0003,
        },
        ModelerPartial {
            pos: (7.9557, -3.0610),
            velocity: (-116.9459, -153.8584),
            acceleration: (-41858.1016, -9474.3242),
            time: 1.0004,
        },
        ModelerPartial {
            pos: (7.9436, -3.0764),
            velocity: (-121.0982, -154.7945),
            acceleration: (-41522.2734, -9360.7930),
            time: 1.0005,
        },
        ModelerPartial {
            pos: (7.9311, -3.0920),
            velocity: (-125.2169, -155.7193),
            acceleration: (-41187.6445, -9247.7998),
            time: 1.0006,
        },
        ModelerPartial {
            pos: (7.9182, -3.1077),
            velocity: (-129.3023, -156.6328),
            acceleration: (-40854.2109, -9135.3506),
            time: 1.0007,
        },
        ModelerPartial {
            pos: (7.9048, -3.1234),
            velocity: (-133.3545, -157.5351),
            acceleration: (-40521.9727, -9023.4395),
            time: 1.0008,
        },
        ModelerPartial {
            pos: (7.8911, -3.1393),
            velocity: (-137.3736, -158.4263),
            acceleration: (-40190.9414, -8912.0703),
            time: 1.0009,
        },
        ModelerPartial {
            pos: (7.8770, -3.1552),
            velocity: (-141.3597, -159.3065),
            acceleration: (-39861.0977, -8801.2402),
            time: 1.0010,
        },
    ];
    assert!(result
        .into_iter()
        .zip(expected)
        .fold(true, |acc, x| { acc && x.0.near(x.1) }));
}
