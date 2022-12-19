#pragma once
#include <vector>
#include <unordered_map>

#include "common.hpp"

struct Edge;
struct Node;

using NodeId = u64;

class GraphStorage {
public:
    bool addNode(NodeId id);
    bool addEdge(NodeId from, NodeId to, u64 weight, bool is_boundary_edge);

private:
    std::vector<Node> graph;
    std::unordered_map<NodeId, Node*> id_node_map;
};
