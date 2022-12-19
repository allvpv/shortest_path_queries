#include <functional>

#include <grpc++/server.h>
#include <grpc++/server_builder.h>

#include "absl/flags/flag.h"
#include "absl/flags/parse.h"
#include "absl/flags/usage.h"
#include "absl/log/initialize.h"
#include "absl/log/log.h"
#include "absl/strings/substitute.h"

#include "protos/graph.grpc.pb.h"
#include "protos/graph.pb.h"

#include "common.hpp"
#include "graph_storage.hpp"

ABSL_FLAG(u16, port, 0, "listen on this port");

class WorkerImpl final : public WorkerService::Service {
    grpc::Status ReceiveGraphDefinition(grpc::ServerContext* context,
        const GraphDefinition* definition, GraphDefinitionConfirmed* confirmation) override {

        LOG(INFO) << "Got graph"
                  << " [id: " << definition->graph_id() << "]";

        this->graph_id = definition->graph_id();
        confirmation->set_graph_id(this->graph_id);

        return {};
    }

    grpc::Status ReceiveGraphPieces(grpc::ServerContext* context,
        grpc::ServerReader<GraphPiece>* reader, GraphConfirmed* response) override {
        GraphPiece piece;

        while (reader->Read(&piece)) {
            LOG(INFO) << "Got piece of graph!\n";

            for (const auto& node : piece.nodes()) {
                u64 id = node.node_id();
                LOG(INFO) << " \t[node id] " << id << "got";
                this->storage.addNode(id);
            }

            for (const auto& edge : piece.edges()) {
                // clang-format off
                LOG(INFO) << " \t[from id] " << edge.node_from_id()
                          << " \t[to id] " << edge.node_to_id()
                          << " \t[weight] " << edge.weight()
                          << std::boolalpha << " \t[is bound] " << edge.is_boundary_edge() << '\n';
                // clang-format on
            }
        }

        response->set_graph_id(this->graph_id);
        return {};
    }

private:
    u64 graph_id;
    GraphStorage storage;
};

int main(int argc, char** argv) {
    absl::SetProgramUsageMessage("Syntax");
    absl::ParseCommandLine(argc, argv);
    absl::InitializeLog();

    auto server_address = absl::Substitute("localhost:$0", absl::GetFlag(FLAGS_port));

    WorkerImpl service;
    grpc::ServerBuilder builder;
    builder.AddListeningPort(server_address, grpc::InsecureServerCredentials());
    builder.RegisterService(&service);

    auto server { builder.BuildAndStart() };

    if (!server) {
        LOG(ERROR) << "Failed to launch server on " << server_address << '\n';
        return EXIT_FAILURE;
    } else {
        LOG(INFO) << "Server listening on " << server_address << '\n';
        server->Wait();
    }
}
