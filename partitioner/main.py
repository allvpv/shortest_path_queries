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
    parser.add_argument("--port", metavar="K", help="Start listening on port K", type=int, required=True)

    args = parser.parse_args()

    parser = OpenStreetMapParser(args.graph)
    quantile_partitioner = QuantilePartitioner()
    partitions = quantile_partitioner.partition(parser, n_partitions=args.n_partitions)

    print(partitions)
    partitions_count = {}

    for _, line in parser.get_lines():
        if parser.is_node_line(line):
            node, lat, lon = parser.get_node_info(line)
            partition_ix, partition = next(filter(lambda p: is_in_partition(lon, lat, p[1]), enumerate(partitions)))
            if partition_ix in partitions_count:
                partitions_count[partition_ix] += 1
            else:
                partitions_count[partition_ix] = 1

    for p in partitions_count:
        print("partition: {}, count: {}".format(p, partitions_count[p]))

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
