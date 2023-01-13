fn main() -> Result<(), std::io::Error> {
    let protos = [
        "../../protos/manager.proto",
        "../../protos/worker.proto",
        "../../protos/executer.proto",
    ];
    let directory = [
        "../../protos/",
        "../../external/com_google_protobuf/_virtual_imports/empty_proto/",
    ];

    tonic_build::configure().compile(&protos, &directory)
}
