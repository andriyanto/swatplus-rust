//! The SWAT+ simulation, translated from the Fortran engine.
//!
//! So far: the clock, the daily weather, and HRUs without management (water balance, one
//! plant, soil nutrients with the original carbon model, erosion). Each function names the Fortran routine it follows,
//! and keeps its order of operations, so results can be compared with SWAT+ value by value.
// Index loops and SWAT+'s own constants (3.1416, 6.283185) are kept as the Fortran writes them,
// so the arithmetic is the same.
#![allow(clippy::needless_range_loop, clippy::approx_constant, clippy::manual_clamp)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable))]

pub mod climate;
pub mod clock;
pub mod fortran;
pub mod hru;
pub mod hyd;
pub mod route;
pub mod run;
