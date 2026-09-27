// Vendored contract source for hermetic `cargo publish --verify` builds.
//
// Source of truth: `proto/algorithco_guard/v0/*.proto` at tag `v0.0.1-alpha`.
// NEVER hand-edit these copies. To update: copy from the tagged proto release,
// keep filenames + `algorithco_guard/v0/` prefix identical so
// `import "algorithco_guard/v0/events.proto"` still resolves.
//
// Why vendored (ADR-0013 prep): `cargo package` only includes files inside the
// crate directory, so `build.rs` cannot reference `../../../proto/` during
// `cargo publish --verify`. `build.rs` compiles from this directory, which is
// byte-identical to the pinned tag.
