import argparse
import grpc
from concurrent import futures

from parsers.open_street_map import OpenStreetMapParser
from partitioners.grid import QuantilePartitioner
from partitioners import is_in_partition
import rpc_servers.manager_pb2_grpc as manager_pb2_grpc
import rpc_servers.manager_server as manager_server


def main():
    parser = argparse.ArgumentParser(description='Parse graphs and compute node regions')
    parser.add_argument("graph", help="Graph input file in OSM XML format (compressed with .gz)")
    parser.add_argument("--n_partitions", type=int, help="Number of graph partitions", required=False, default=16)
    parser.add_argument("--ports", metavar="K", help="connect to workers on ports [K, K+N_PARTITIONS) ", required=True)

    args = parser.parse_args()

    parser = OpenStreetMapParser(args.graph)
    quantile_partitioner = QuantilePartitioner()
    partitions = quantile_partitioner.partition(parser, n_partitions=args.n_partitions)

    print(partitions)
    for _, line in parser.get_lines():
        if parser.is_node_line(line):
            node, lat, lon = parser.get_node_info(line)
            partition_ix, partition = next(filter(lambda p: is_in_partition(lon, lat, p[1]), enumerate(partitions)))
            print(f"Node: id={node}, latitude={lat}, longitude={lon}, partition_ix={partition_ix}, partition={partition}")

    # Serve
    print('Starting the server...')
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    manager_pb2_grpc.add_ManagerServiceServicer_to_server(
        manager_server.ManagerServiceServicer(partitions, parser), server
    )
    server.add_insecure_port('[::]:50051')
    server.start()
    server.wait_for_termination()


if __name__ == "__main__":
    main()
