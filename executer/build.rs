fn main() {
    tonic_build::configure()
        .compile(
            &[
                "../protos/worker.proto",
                "../protos/manager.proto",
                "../protos/executer.proto",
            ],
            &["../protos/"],
        )
        .unwrap();
}
