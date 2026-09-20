use std::path::PathBuf;

fn main() {
    // Use the vendored protoc to avoid host `protoc` requirement.
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("protoc-bin-vendored failed");
    std::env::set_var("PROTOC", &protoc);

    let proto_root = PathBuf::from("../../../proto");
    let protos = [
        proto_root.join("algorithco_guard/v0/events.proto"),
        proto_root.join("algorithco_guard/v0/decision.proto"),
        proto_root.join("algorithco_guard/v0/dataset.proto"),
    ];
    // Tell cargo to rerun if protos change.
    for p in &protos {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    println!("cargo:rerun-if-changed=../../../proto/buf.yaml");

    let mut config = prost_build::Config::new();
    config.protoc_arg("--experimental_allow_proto3_optional");
    // prost_types for google.protobuf.Timestamp
    config.compile_protos(&protos, &[proto_root]).expect("prost_build failed");
}
