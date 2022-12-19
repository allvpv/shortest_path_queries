#include "absl/log/log.h"
#include "absl/log/check.h"
#include "graph_storage.hpp"

struct Node {
    u64 id;
    std::vector<Edge> edges;
};

struct Edge {
    u64 weight;
    Node* to;
};

bool GraphStorage::addNode(NodeId id) {
    this->graph.emplace_back(Node {
        .id = id,
    });

    auto [_, inserted] = this->id_node_map.insert({id, &this->graph.back()});

    if (!inserted) {
        LOG(ERROR) << "Something is wrong! Adding node with the same id twice.";
        this->graph.pop_back();
        return false;
    } else {
        LOG(INFO) << "Added node, id: " << id;
        return true;
    }
}

bool GraphStorage::addEdge(NodeId from, NodeId to, u64 weight, bool is_boundary_edge) {
    auto get_from_id = [this](NodeId id) -> Node* {
        auto it = this->id_node_map.find(id);

        if (it == this->id_node_map.end()) {
            LOG(ERROR) << "Error. Canot find [id: " << id << "] node, but edge contains it";
            return nullptr;
        }

        return it->second;
    };

    Node* from_ptr = get_from_id(from);
    Node* to_ptr = get_from_id(to);

    from_ptr->edges.emplace_back(Edge {
        .weight = weight,
        .to = to_ptr,
    });

    if (!from_ptr || !to_ptr)
        return false;
    else
        return true;
}

