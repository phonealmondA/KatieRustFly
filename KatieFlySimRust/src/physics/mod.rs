// Physics simulation module

pub mod gravity_simulator;
// Advanced trajectory system (work in progress - requires API extensions)
// pub mod trajectory;

pub use gravity_simulator::{GravitySimulator, orbital};
