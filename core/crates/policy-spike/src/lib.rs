//! Policy-language spike ONLY (not product code).
//! CEL (`cel` crate) vs minimal pest DSL over synthetic shell facts.
//!
//! NOT product code: `algo-policy` and all product crates must NOT depend
//! on this crate. Fail-safe everywhere: any error → Ask, never Allow.

pub mod cel_engine;
pub mod corpus;
pub mod dsl;
pub mod facts;
