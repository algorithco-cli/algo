use std::path::PathBuf;

fn main() {
    // Use the vendored protoc to avoid host `protoc` requirement.
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("protoc-bin-vendored failed");
    std::env::set_var("PROTOC", &protoc);

    // Vendored contract source (see `proto/README.md` in this crate): `cargo
    // package` only ships files inside the crate, so building from
    // `../../../proto/` fails `cargo publish --verify`. These copies are
    // byte-identical to proto tag `v0.0.1-alpha` — never hand-edit, re-vendor
    // from the tagged release to update.
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto");
    let protos = [
        proto_root.join("algorithco_guard/v0/events.proto"),
        proto_root.join("algorithco_guard/v0/decision.proto"),
        proto_root.join("algorithco_guard/v0/dataset.proto"),
    ];
    // Tell cargo to rerun if protos change.
    for p in &protos {
        println!("cargo:rerun-if-changed={}", p.display());
    }

    let mut config = prost_build::Config::new();
    config.protoc_arg("--experimental_allow_proto3_optional");
    // prost_types for google.protobuf.Timestamp
    config
        .compile_protos(&protos, &[proto_root])
        .expect("prost_build failed");
}
