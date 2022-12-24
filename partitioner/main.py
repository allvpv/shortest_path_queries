import argparse
import gzip
import grpc

from parsers.open_street_map import OpenStreetMapParser
from partitioners.grid import GridPartitioner
from protos.manager_pb2_grpc import *

def main():
    parser = argparse.ArgumentParser(description='Parse graphs and compute node regions')
    parser.add_argument("graph", help="Graph input file in OSM XML format (compressed with .gz)")
    parser.add_argument("--n_partitions", type=int, help="Number of graph partitions", required=False, default=16)
    parser.add_argument("--ports", metavar="K", help="connect to workers on ports [K, K+N_PARTITIONS) ", required=True)

    args = parser.parse_args()

    parser = OpenStreetMapParser(args.graph)
    qp = GridPartitioner()
    partitions = qp.partition(parser, n_partitions=args.n_partitions)

    print(partitions)

    def is_in_partition(lon, lat, partition):
        x_min, x_max = partition[0]
        y_min, y_max = partition[1]
        return x_min < lon <= x_max and y_min < lat <= y_max

    for _, line in parser.get_lines():
        if parser.is_node_line(line):
            node, lat, lon = parser.get_node_info(line)
            partition_ix, partition = next(filter(lambda p: is_in_partition(lon, lat, p[1]), enumerate(partitions)))
            print(f"Node: id={node}, latitude={lat}, longitude={lon}, partition_ix={partition_ix}, partition={partition}")

if __name__ == "__main__":
    main()
