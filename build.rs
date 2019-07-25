extern crate prost_build;

pub fn main() {
    let mut config = prost_build::Config::new();
    config
        .compile_protos(
            &[
                "proto/myorg.mything.v1.proto",
            ],
            &["proto"],
        )
        .unwrap();
}
