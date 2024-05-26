// imports
use std::collections::VecDeque;

#[cfg(test)]
extern crate approx;

// modules
mod engine;
pub mod input;
pub mod params;
mod position_modeler;
pub mod results;
mod state_modeler;
mod utils;

pub use engine::StrokeModeler;
pub use input::ModelerInput;
pub use input::ModelerInputEventType;
pub use params::ModelerParams;
use position_modeler::PositionModeler;
use results::ModelerPartial;
pub use results::ModelerResult;
use state_modeler::StateModeler;
