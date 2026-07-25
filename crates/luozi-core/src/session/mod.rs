//! Pure session state machine and delivery routing. No I/O.

mod delivery;
mod machine;

pub use delivery::{route_delivery, DeliveryAction, TargetValidation};
pub use machine::{
    SessionCommand, SessionEffect, SessionId, SessionMachine, SessionPhase, MIN_RECORDING_MS,
};
