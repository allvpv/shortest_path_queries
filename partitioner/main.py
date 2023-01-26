import argparse
import grpc
from concurrent import futures

from parsers.open_street_map import OpenStreetMapParser
from partitioners.grid import QuantilePartitioner
from partitioners import is_in_partition
import rpc_servers.manager_pb2_grpc as manager_pb2_grpc
import rpc_servers.manager_server as manager_server


def main():
    print('Parsing args...')
    parser = argparse.ArgumentParser(description='Parse graphs and compute node regions')
    parser.add_argument("graph", help="Graph input file in OSM XML format (compressed with .gz)")
    parser.add_argument("--n_partitions", type=int, help="Number of graph partitions", required=False, default=16)
    parser.add_argument("--port", metavar="K", help="Start listening on port K", type=int, required=True)

    args = parser.parse_args()

    print('Creating partitioning...')
    parser = OpenStreetMapParser(args.graph)
    quantile_partitioner = QuantilePartitioner()
    partitions = quantile_partitioner.partition(parser, n_partitions=args.n_partitions)

    print('Created partitioning:')
    print(partitions)

    # Serve
    print('Starting the server...')
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    manager_pb2_grpc.add_ManagerServiceServicer_to_server(
        manager_server.ManagerServiceServicer(partitions, parser), server
    )
    server.add_insecure_port(f'[::]:{args.port}')
    server.start()
    print('Started the server.')
    server.wait_for_termination()


if __name__ == "__main__":
    main()
