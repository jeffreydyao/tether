//! # tether-server
//!
//! HTTP server library for the tether phone proximity tracking system.
//!
//! This library provides the API handlers and state management for tether.

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod api;
pub mod logging;
pub mod state;
