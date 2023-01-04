from partitioners.partitioner import Partitioner
from parsers.parser import GraphParser
import numpy as np
import itertools
from collections import defaultdict
from typing import Tuple, List

class GridPartitioner(Partitioner):
    def partition(self, parser: GraphParser, n_partitions: int) -> List[List[Tuple[float]]]:
        x_min, x_max = None, None
        y_min, y_max = None, None
        for _, line in parser.get_lines():
            if parser.is_node_line(line):
                _, lat, lon = parser.get_node_info(line)
                x_min = min(lon, x_min) if x_min else lon
                x_max = max(lon, x_max) if x_max else lon
                y_min = min(lat, y_min) if y_min else lat
                y_max = max(lat, y_max) if y_max else lat

        # TODO: co jeśli nie będzie idealnego pierwiastka
        n_side_partitions = int(n_partitions ** 0.5)

        # Decrement minimums so that the smallest element has a partition as well
        x_bin_bounds = np.linspace(x_min - 1, x_max, num=n_side_partitions+1)
        y_bin_bounds = np.linspace(y_min - 1, y_max, num=n_side_partitions+1)

        return [[x_bin, y_bin] for x_bin, y_bin in itertools.product(zip(x_bin_bounds, x_bin_bounds[1:]), zip(y_bin_bounds, y_bin_bounds[1:]))]

class QuantilePartitioner(Partitioner):
    def sample_stream(self, parser: GraphParser, n_partitions: int, n_buckets: int, n_sample: int):
        def flat_len(x):
            return sum(len(l) for l in x.values())
        n_side_partitions = int(np.round(n_partitions ** 0.5))
        lon_sample = defaultdict(lambda: [])
        lat_sample = defaultdict(lambda: [])
        lon_min, lon_max = None, None
        lat_min, lat_max = None, None
        n_lat_accepted_buckets = n_buckets
        n_lon_accepted_buckets = n_buckets
        for _, line in parser.get_lines():
            if parser.is_node_line(line):
                _, lat, lon = parser.get_node_info(line)
                lon_min = min(lon, lon_min) if lon_min else lon
                lon_max = max(lon, lon_max) if lon_max else lon
                lat_min = min(lat, lat_min) if lat_min else lat
                lat_max = max(lat, lat_max) if lat_max else lat
                if hash(lat) % n_buckets < n_lat_accepted_buckets:
                    lat_sample[hash(lat) % n_buckets].append(lat)
                if hash(lon) % n_buckets < n_lon_accepted_buckets:
                    lon_sample[hash(lon) % n_buckets].append(lon)

                while flat_len(lat_sample) > 2 * n_sample:
                    n_lat_accepted_buckets -= 1
                    lat_sample = {k: v for k, v in lat_sample.items() if k < n_lat_accepted_buckets}
                while flat_len(lon_sample) > 2 * n_sample:
                    n_lon_accepted_buckets -= 1
                    lon_sample = {k: v for k, v in lon_sample.items() if k < n_lon_accepted_buckets}

        lat_sample = [x for l in lat_sample.values() for x in l]
        lon_sample = [x for l in lon_sample.values() for x in l]
        assert len(lat_sample) > 0
        assert len(lon_sample) > 0
        quantiles = np.linspace(0, 1., n_side_partitions + 1)
        self.lon_quantiles = np.quantile(lon_sample, quantiles)
        self.lat_quantiles = np.quantile(lat_sample, quantiles)
        self.lon_quantiles[0] = lon_min - 1
        self.lon_quantiles[-1] = lon_max
        self.lat_quantiles[0] = lat_min - 1
        self.lat_quantiles[-1] = lat_max

    def partition(self, parser: GraphParser, n_partitions: int) -> List[List[Tuple[float]]]:
        self.sample_stream(parser, n_partitions, n_buckets=100, n_sample=1000)
        return [[x_bin, y_bin] for x_bin, y_bin in itertools.product(zip(self.lon_quantiles, self.lon_quantiles[1:]), zip(self.lat_quantiles, self.lat_quantiles[1:]))]

