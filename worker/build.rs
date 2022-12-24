fn main() {
    tonic_build::configure()
        .compile(&["../protos/manager.proto"], &["../protos/"])
        .unwrap();

    tonic_build::configure()
        .compile(&["../protos/worker.proto"], &["../protos/"])
        .unwrap();
}
