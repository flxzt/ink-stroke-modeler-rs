// tests

#[cfg(test)]
mod ink_stroke_modeler {

    use crate::{
        impl_ds::compare_results,
        utils::{interp, interp2, nearest_point_on_segment, normalize01_32},
    };

    // import parent
    use super::super::*;
    #[test]
    fn validation_modeler_params() {
        let s = ModelerParams::new(-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 0, 0, 0);
        println!("{:?}", s); // use --nocapture to show the result here
                             // verify that we actually have an error
        match s {
            Ok(_) => assert!(false),
            Err(_) => assert!(true),
        }
    }

    #[test]
    fn test_clamping() {
        // testing that the clamp does the action we expect in rust
        assert_eq!((-2.0 as f64).clamp(0.0, 1.0), 0.0);
        assert_eq!((0 as f64).clamp(0.0, 1.0), 0.0);
        assert_eq!((0.3 as f64).clamp(0.0, 1.0), 0.3);
        assert_eq!((0.7 as f64).clamp(0.0, 1.0), 0.7);
        assert_eq!((1.0 as f64).clamp(0.0, 1.0), 1.0);
        assert_eq!((1.1 as f64).clamp(0.0, 1.0), 1.0);
    }

    #[test]
    fn test_normalize_float() {
        assert_eq!(normalize01_32(1., 2., 1.5), 0.5);
        assert_eq!(normalize01_32(7., 3., 4.), 0.75); //should also work in reverse
        assert_eq!(normalize01_32(-1., 1., 2.), 1.);
        assert_eq!(normalize01_32(1., 1., 1.), 0.);
        assert_eq!(normalize01_32(1., 1., 0.), 0.);
        assert_eq!(normalize01_32(1., 1., 2.), 1.);
    }

    #[test]
    fn test_inter_float() {
        assert_eq!(interp(5.0, 10.0, 0.2), 6.0);
        assert_eq!(interp(10.0, -2.0, 0.75), 1.0);
        assert_eq!(interp(-1.0, 2.0, -3.0), -1.0);
        assert_eq!(interp(5.0, 7.0, 20.0), 7.0);
    }

    #[test]
    fn test_interp_vec2() {
        assert_eq!(interp2((1.0, 2.0), (3.0, 5.0), 0.5), (2.0, 3.5));
        assert_eq!(interp2((-5.0, 5.0), (-15.0, 0.0), 0.4), (-9.0, 3.0));
        assert_eq!(interp2((7.0, 9.0), (25.0, 30.0), -0.1), (7.0, 9.0));
        assert_eq!(interp2((12.0, 5.0), (13.0, 14.0), 3.2), (13.0, 14.0));
    }

    #[test]
    fn test_nearest_point() {
        assert_eq!(
            nearest_point_on_segment((0.0, 0.0), (1.0, 0.0), (0.25, 0.5),),
            0.25
        );
        assert_eq!(
            nearest_point_on_segment((3.0, 4.0), (5.0, 6.0), (-1.0, -1.0),),
            0.0
        );
        assert_eq!(
            nearest_point_on_segment((20.0, 10.0), (10.0, 5.0), (2.0, 2.0),),
            1.0
        );
        assert_eq!(
            nearest_point_on_segment((0.0, 5.0), (5.0, 0.0), (3.0, 3.0),),
            0.5
        );

        // degenerate cases
        assert_eq!(
            nearest_point_on_segment((0.0, 0.0), (0.0, 0.0), (5.0, 10.0),),
            0.0
        );
        assert_eq!(
            nearest_point_on_segment((3.0, 7.0), (3.0, 7.0), (0.0, -20.0),),
            0.0
        );
    }

    #[test]
    fn test_wobble_smoother_line() {
        // need to create a StrokeModeler
        let mut new_modeler = StrokeModeler::default();
        new_modeler.wobble_update(&ModelerInput {
            event_type: ModelerInputEventType::kDown,
            pos: (3., 4.),
            time: 1.0,
            pressure: 0.0,
        });
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (3.016, 4.),
                time: 1.016,
                pressure: 0.0,
            }),
            (3.016, 4.)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (3.032, 4.),
                time: 1.032,
                pressure: 0.0,
            }),
            (3.024, 4.)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (3.048, 4.),
                time: 1.048,
                pressure: 0.0,
            }),
            (3.032, 4.)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
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
            event_type: ModelerInputEventType::kDown,
            pos: (1., 2.),
            time: 5.0,
            pressure: 0.0,
        });
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (1.016, 2.),
                time: 5.016,
                pressure: 0.0,
            }),
            (1.016, 2.0)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (1.016, 2.016),
                time: 5.032,
                pressure: 0.0,
            }),
            (1.016, 2.008)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (1.032, 2.016),
                time: 5.048,
                pressure: 0.0,
            }),
            (1.02133, 2.01067)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (1.032, 2.032),
                time: 5.064,
                pressure: 0.0,
            }),
            (1.0266667, 2.0213333)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (1.048, 2.032),
                time: 5.080,
                pressure: 0.0,
            }),
            (1.0373333, 2.0266667)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
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
                event_type: ModelerInputEventType::kMove,
                pos: (7., 3.024),
                time: 8.016,
                pressure: 0.0,
            }),
            (7.0, 3.024)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (7.024, 3.024),
                time: 8.032,
                pressure: 0.0,
            }),
            (7.024, 3.024)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (7.024, 3.048),
                time: 8.048,
                pressure: 0.0,
            }),
            (7.024, 3.048)
        ));
        assert!(util_compare_floats(
            new_modeler.wobble_update(&ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (7.048, 3.048),
                time: 8.064,
                pressure: 0.0,
            }),
            (7.048, 3.048)
        ));
    }

    fn util_compare_floats(a1: (f32, f32), a2: (f32, f32)) -> bool {
        return (a1.0 - a2.0).abs() < 0.0001 && (a1.1 - a2.1).abs() < 0.0001;
    }

    #[test]
    fn input_test() {
        let mut modeler = StrokeModeler::new(ModelerParams::suggested());

        let inputs = vec![
            ModelerInput {
                event_type: ModelerInputEventType::kDown,
                pos: (0.0, 0.0),
                time: 0.0,
                pressure: 0.1,
            },
            ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (1.0, 0.0),
                time: 0.02,
                pressure: 0.3,
            },
            ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (2.0, 0.0),
                time: 0.04,
                pressure: 0.5,
            },
            ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (2.5, 1.0),
                time: 0.06,
                pressure: 0.8,
            },
            ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (3.0, 1.5),
                time: 0.12,
                pressure: 0.9,
            },
            ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (4.0, 2.0),
                time: 0.13,
                pressure: 0.8,
            },
            ModelerInput {
                event_type: ModelerInputEventType::kMove,
                pos: (3.8, 2.1),
                time: 0.14,
                pressure: 0.7,
            },
            ModelerInput {
                event_type: ModelerInputEventType::kUp,
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
        let mut engine = StrokeModeler::new(ModelerParams::suggested());
        assert!(engine.predict().is_err());
    }

    #[test]
    fn test_singleinput() {
        let mut engine = StrokeModeler::default();
        engine
            .update(ModelerInput {
                pos: (4.0, 5.0),
                event_type: ModelerInputEventType::kDown,
                time: 2.0,
                pressure: 1.0,
            })
            .unwrap();
        assert_eq!(engine.predict().unwrap().len(), 0);
    }

    // tests for the stroke modeler
    #[test]
    fn input_rate_slower() {
        let delta_time = 1. / 30.;
        let mut time = 0.0;
        let mut engine = StrokeModeler::default();

        let first_iter = engine.update(ModelerInput {
            event_type: ModelerInputEventType::kDown,
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
                    event_type: ModelerInputEventType::kMove,
                    pos: (3.2, 4.2),
                    time: time,
                    pressure: 1.0
                })
                .unwrap(),
            vec![
            //ModelerResult {}
            ]
        ));
    }

    #[test]
    fn reset_keep_params() {
        let input = ModelerInput {
            event_type: ModelerInputEventType::kDown,
            pos: (3.0, 4.0),
            time: 0.0,
            pressure: 1.0,
        };
        let mut engine = StrokeModeler::default();
        assert!(engine.reset().is_ok());
        assert!(engine.update(input).is_ok());

        assert!(engine.reset_w_params(ModelerParams::suggested()).is_ok());
        assert!(engine.update(input).is_ok());
    }
}
