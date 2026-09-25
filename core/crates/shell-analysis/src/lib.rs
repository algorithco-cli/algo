//! Shell analysis — tree-sitter-bash → ParsedCmd + facts + obfuscation
//! flags for policy matching.

pub mod facts;
pub mod obfuscation;
pub mod parse;

pub use facts::Facts;
pub use obfuscation::ObfuscationFlags;
pub use parse::{ParseError, ParsedCmd, ParsedCommand};
