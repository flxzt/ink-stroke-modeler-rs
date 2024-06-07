// Modules
mod engine;
mod error;
mod input;
mod params;
mod position_modeler;
mod results;
mod state_modeler;
mod utils;

#[cfg(test)]
extern crate approx;

// Re-Exports
pub use engine::StrokeModeler;
pub use error::ModelerError;
pub use input::ModelerInput;
pub use input::ModelerInputEventType;
pub use params::ModelerParams;
pub use results::ModelerResult;
