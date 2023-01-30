use generated::worker::{
    response_djikstra::{
        MessageType::{
            NewForeignNode as NewForeignNodeVariant,
            SmallestDomesticNode as SmallestDomesticNodeVariant, Success as SuccessVariant,
        },
        NewForeignNode, SmallestDomesticNode, Success,
    },
    NodePointer, ResponseDjikstra,
};

use crate::graph_store::{NodeId, ShortestPathLen, WorkerId};

pub fn new_foreign_node(
    node_id: NodeId,
    worker_id: WorkerId,
    parent_node_id: NodeId,
    shortest_path_len: ShortestPathLen,
) -> ResponseDjikstra {
    ResponseDjikstra {
        message_type: Some(NewForeignNodeVariant(NewForeignNode {
            this_node: Some(NodePointer { worker_id, node_id }),
            parent_node_id,
            shortest_path_len,
        })),
    }
}

pub fn domestic_smallest_node(shortest_path_len: ShortestPathLen) -> ResponseDjikstra {
    ResponseDjikstra {
        message_type: Some(SmallestDomesticNodeVariant(SmallestDomesticNode {
            shortest_path_len,
        })),
    }
}

pub fn success(node_id: NodeId, shortest_path_len: ShortestPathLen) -> ResponseDjikstra {
    ResponseDjikstra {
        message_type: Some(SuccessVariant(Success {
            node_id,
            shortest_path_len,
        })),
    }
}
