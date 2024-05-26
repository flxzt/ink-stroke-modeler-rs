/// modeler Input event Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
#[allow(unused)]
pub enum ModelerInputEventType {
    /// For the first pen down event (pen touches the screen)
    kDown,
    /// For events between the first (`kDown`) and last (`Up`) event (pen moving on the screen)
    kMove,
    /// For the last event (pen going up)
    kUp,
}

/// struct holding all information for input event
#[derive(Clone, Debug, PartialEq)]
pub struct ModelerInput {
    pub event_type: ModelerInputEventType,
    pub pos: (f32, f32),
    pub time: f64,
    pub pressure: f32,
    // tilt and orientation are optional parameters, so we remove them here to
    // make our lives easier
}

impl Default for ModelerInput {
    fn default() -> Self {
        Self {
            event_type: ModelerInputEventType::kDown,
            pos: (0.0, 0.0),
            time: 0.0,
            pressure: 1.0,
        }
    }
}
