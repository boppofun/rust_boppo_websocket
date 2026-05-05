//! # WebSocket client library for Boppo tablets.
//!
//! WebSocket client library for the [Boppo Tablet](https://developer.boppo.com/)
//! for off-device programmable activities and control.
//!
//! ## [`client`]
//!
//! Use [`client::connect`] to get a [`client::BoppoSender`] /
//! [`client::BoppoReceiver`] pair. Because each connection is independent, you
//! can connect to multiple tablets simultaneously without any global state.
//!
//! ## Globals Setup
//!
//! [`connect_and_setup_globals`] and [`setup_globals`] wire one client
//! connection into [`boppo_core`]'s global HAL, enabling high-level APIs like
//! [`Button::B0.set_color(...)`][crate::Button::set_color] and the top-level
//! functions in this crate ([`audio::play`], [`audio::stop_all_sounds`], etc.).
//!
//! Most functions outside of [`client`] require [`setup_globals`] to have been
//! called first and panic otherwise.
//!
//! # Examples
//!
//! Two examples are in the `examples/` directory:
//!
//! - **`simple`** — buttons light up blue on press and play a snare
//! - **`features`** — all buttons wired to lights, music, sound effects, and playback control
#![deny(missing_docs)]

pub mod audio;
pub mod client;
pub mod commands;
mod error;
mod globals;

pub use boppo_core::*;
pub use error::Error;
pub use globals::{connect_and_setup_globals, setup_globals};
