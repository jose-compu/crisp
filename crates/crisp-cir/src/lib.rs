//! CIR — typed IR with resolved ownership modes (spec §17.1).
//!
//! Synthesizes shape traits, Box insertion, default-field builders, clone materialization.

pub mod node;

pub struct CirBuilder;
