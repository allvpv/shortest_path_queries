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

