#include <functional>
#include <ios>
#include <iostream>

#include <boost/program_options.hpp>
#include <glog/logging.h>
#include <grpc++/server.h>
#include <grpc++/server_builder.h>

#include "protos/graph.grpc.pb.h"
#include "protos/graph.pb.h"
#include "worker.hpp"

namespace po = boost::program_options;

class WorkerImpl final : public WorkerService::Service {
    grpc::Status ReceiveGraphDefinition(grpc::ServerContext* context,
        const GraphDefinition* definition, GraphDefinitionConfirmed* confirmation) override {

        LOG(INFO) << "Got graph:\n"
                  << " \t[id] " << definition->graph_id() << " \t[edges cnt] "
                  << definition->edges_count() << '\n';

        return {};
    }

    grpc::Status ReceiveGraphPieces(grpc::ServerContext* context,
        grpc::ServerReader<GraphPiece>* reader, GraphConfirmed* response) override {
        GraphPiece piece;

        while (reader->Read(&piece)) {
            LOG(INFO) << "Got piece of graph!\n";

            for (const auto& node : piece.nodes()) {
                LOG(INFO) << " \t[node id] " << node.node_id() << '\n';
            }

            for (const auto& edge : piece.edges()) {
                LOG(INFO) << " \t[edge id] " << edge.edge_id() << " \t[from id] "
                          << edge.node_from_id() << " \t[to id] " << edge.node_to_id()
                          << " \t[weight] " << edge.weight() << std::boolalpha << " \t[is bound] "
                          << edge.is_boundary_edge() << '\n';
            }
        }

        return {};
    }
};

po::variables_map get_options(int argc, char** argv) {
    po::options_description desc { "Syntax" };
    po::variables_map options;

    // clang-format off
    desc.add_options()
      ("port,p", po::value<u16>()->value_name("<u16>")->required(), "")
      ("help,h", "prints this message");
    // clang-format on

    po::store(po::parse_command_line(argc, argv, desc), options);

    if (options.count("help")) {
        std::cout << desc << "\n=";
        return options;
    }

    po::notify(options);
    return options;
}

int main(int argc, char** argv) {
    try {
        google::InitGoogleLogging(argv[0]);

        po::variables_map options = get_options(argc, argv);

        if (options.count("help"))
            return EXIT_SUCCESS;

        std::string server_address = [&] {
            std::ostringstream oss;
            oss << "localhost:" << options["port"].as<u16>();
            return oss.str();
        }();

        WorkerImpl service;

        grpc::ServerBuilder builder;
        builder.AddListeningPort(server_address, grpc::InsecureServerCredentials());
        builder.RegisterService(&service);

        auto server { builder.BuildAndStart() };

        if (!server) {
            LOG(ERROR) << "Failed to launch server on " << server_address << '\n';
        }  else {
            LOG(INFO) << "Server listening on " << server_address << '\n';
            server->Wait();
        }

    } catch (po::error_with_option_name& e) {
        std::cerr << "Error: " << e.what() << ".\n";
        std::cerr << "Use `--help` to learn more.\n";
        return 1;

    } catch (std::runtime_error& e) {
        std::cerr << "Error: " << e.what() << ".\n";
        return 1;
    }
}
