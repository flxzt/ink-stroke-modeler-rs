// imports
use std::collections::VecDeque;
extern crate approx;

// modules
mod engine;
mod impl_ds;
pub mod input;
pub mod params;
mod position_modeler;
pub mod results;
mod state_modeler;
mod testing;
mod utils;

pub use input::ModelerInput;
pub use input::ModelerInputEventType;
pub use params::ModelerParams;
use position_modeler::PositionModeler;
use results::ModelerPartial;
pub use results::ModelerResult;
use state_modeler::StateModeler;
pub use engine::StrokeModeler;