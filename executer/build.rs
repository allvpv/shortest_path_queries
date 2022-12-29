fn main() {
    tonic_build::configure()
        .compile(
            &[
                "../GraphWorker/protos/worker.proto",
                "../GraphWorker/protos/manager.proto",
                "../GraphWorker/protos/executer.proto",
            ],
            &["../GraphWorker/protos/"],
        )
        .unwrap();
}
