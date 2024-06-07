#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ElementError {
    #[error("A duplicate element is sent to the modeler")]
    Duplicate,
    #[error("A sent element has a time earlier than the previous one")]
    NegativeTimeDelta,
    #[error("Sent element order is incorrect")]
    Order {
        #[from]
        src: ElementOrderError,
    },
    #[error("Sent element's time is too far apart from the previous one.")]
    TooFarApart,
}

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
#[allow(clippy::enum_variant_names)]
pub enum ElementOrderError {
    #[error("Down Event is not the first or occured after a different event")]
    UnexpectedDown,
    #[error("Move event occured before a initial down event")]
    UnexpectedMove,
    #[error("No other event occured before an up event")]
    UnexpectedUp,
}

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum ModelerError {
    #[error("Input element error")]
    Element {
        #[from]
        src: ElementError,
    },
}
