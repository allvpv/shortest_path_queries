from partitioners.partitioner import Partitioner
from parsers.parser import GraphParser
import numpy as np
import itertools
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
    def sample_stream(self, parser: GraphParser, n_partitions: int, n_buckets: int):
        n_side_partitions = int(np.round(n_partitions ** 0.5))
        lon_sample = []
        lat_sample = []
        lon_min, lon_max = None, None
        lat_min, lat_max = None, None
        for _, line in parser.get_lines():
            if parser.is_node_line(line):
                _, lat, lon = parser.get_node_info(line)
                lon_min = min(lon, lon_min) if lon_min else lon
                lon_max = max(lon, lon_max) if lon_max else lon
                lat_min = min(lat, lat_min) if lat_min else lat
                lat_max = max(lat, lat_max) if lat_max else lat
                if hash(lat) % n_buckets < n_side_partitions:
                    lat_sample.append(lat)
                if hash(lon) % n_buckets < n_side_partitions:
                    lon_sample.append(lon)

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
        self.sample_stream(parser, n_partitions, n_buckets=n_partitions ** 2)
        return [[x_bin, y_bin] for x_bin, y_bin in itertools.product(zip(self.lon_quantiles, self.lon_quantiles[1:]), zip(self.lat_quantiles, self.lat_quantiles[1:]))]

