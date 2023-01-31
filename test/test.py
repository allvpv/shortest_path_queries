import argparse
import grpc

import grpc
import executer_pb2
import executer_pb2_grpc
from plot_utils import get_path_plot

def forget_query(stub, query_id):
    stub.ForgetQuery(executer_pb2.QueryId(query_id=query_id))


def main():
    parser = argparse.ArgumentParser(description='Test the implementation')
    parser.add_argument("--from-node", type=int, help="",required=True)
    parser.add_argument("--to-node", type=int, help="",required=True)
    parser.add_argument("--executer-addr", help="", required=True)
    args = parser.parse_args()

    with grpc.insecure_channel(args.executer_addr) as channel:
        stub = executer_pb2_grpc.ExecuterStub(channel)

        print("Trying to get shortest path from {} to {} ...".format(args.from_node, args.to_node))

        response = stub.ShortestPathQuery(executer_pb2.QueryData(node_id_from=args.from_node, node_id_to=args.to_node))

        if not response.HasField("shortest_path_len"):
            print("Path not found")

            if response.HasField("query_id"):
                print("Sending forget query")
                forget_query(stub, response.query_id)

            return

        print(f"Shortest path length received: {response.shortest_path_len}")

        if not response.HasField("query_id"):
            print("Query not established")
            return

        input('press enter to get path')

        print("Path found:")
        nodes = []

        for node in stub.BacktrackPathForQuery(executer_pb2.QueryId(query_id=response.query_id)):
            print(f"Node id: {node.node_id}, Worker id: {node.worker_id}")
            nodes.append(node)

        input('press enter to get coords')

        coords = []
        for node, resp in zip(nodes, stub.GetCoordinates(iter(nodes))):
            print(f"Node id: {node.node_id}, lat: {resp.lat}, lon: {resp.lon}")
            coords.append((resp.lat, resp.lon))

        path_plot = get_path_plot(coords)
        path_plot.show()

        input('press enter to forget query')

        forget_query(stub, response.query_id)

if __name__ == "__main__":
    main()
