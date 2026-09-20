//! Shell analysis — tree-sitter-bash → ParsedCmd + facts + obfuscation
//! Spec: `plans/phase-1-03-core-shell-analysis.md`

pub mod facts;
pub mod obfuscation;
pub mod parse;

pub use parse::{ParsedCmd, ParsedCommand, ParseError};
pub use facts::Facts;
pub use obfuscation::ObfuscationFlags;
