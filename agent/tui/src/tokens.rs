//! Compile-time projection of the canonical root `design-tokens.css`.

#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/design_tokens.rs"));
