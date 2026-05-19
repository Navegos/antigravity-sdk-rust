#![allow(clippy::unwrap_used)]

fn main() {
    let descriptor_path =
        std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("proto_descriptor.bin");

    let mut config = prost_build::Config::new();
    config
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(&["proto/localharness.proto"], &["proto/"])
        .unwrap();

    let descriptor_set = std::fs::read(descriptor_path).unwrap();
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)
        .unwrap()
        .build(&[".antigravity.localharness"])
        .unwrap();
}
