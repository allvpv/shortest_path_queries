from parsers.parser import GraphParser
from typing import Text, Tuple, List, Dict
import re
import gzip

class OpenStreetMapParser(GraphParser):
    def __init__(self, graph_path: Text):
        self.graph_path = graph_path
        self.node_regex = re.compile(r'\s<node.*')
        self.id_regex = re.compile(r'id="(\d+)"')
        self.lat_regex = re.compile(r'lat="([\d\.]+)"')
        self.lon_regex = re.compile(r'lon="([\d\.]+)"')

    def get_lines(self) -> Tuple[int, Text]:
        with gzip.open(self.graph_path, "r") as f:
            for ix, line in enumerate(f):
                yield ix, line.decode('utf-8')

    def is_node_line(self, line: Text) -> bool:
        return self.node_regex.match(line) is not None

    def get_node_info(self, line: Text) -> Tuple[int, float, float]:
        id_match = self.id_regex.search(line)
        lat_match = self.lat_regex.search(line)
        lon_match = self.lon_regex.search(line)

        return int(id_match.group(1)), float(lat_match.group(1)), float(lon_match.group(1))

    def get_edge_info(self, line: Text) -> Tuple[int, int, float]:
        pass

    def is_in_partition(lon, lat, partition):
        x_min, x_max = partition[0]
        y_min, y_max = partition[1]
        return x_min < lon <= x_max and y_min < lat <= y_max

    def parse_way(way_lines):
        tag_regex = re.compile(r'\s*<tag k="(\w+)" v="(.+)"\s*\/>')
        node_regex = re.compile(r'\s*<nd ref="(\d+)"\s*\/>')
        way_begin = re.compile(r'\s*<way [^>]*>\s*')
        way_end = re.compile(r'\s*<\/way>\s*')

        assert way_begin.match(way_lines[0])
        assert way_end.match(way_lines[-1])

        tag_dict = {}
        node_list = []
        for line in way_lines:
            if node_regex.match(line):
                match = node_regex.match(line)
                node_list.append(int(match.group(1)))
            elif tag_regex.match(line):
                match = tag_regex.match(line)
                tag_dict[match.group(1)] = match.group(2)

        return node_list, tag_dict

    def get_partition_nodes(self, partitions: List[List[Tuple[float]]], partition_ix: int) -> Dict[int, Tuple[float, float]]:
        partition = partitions[partition_ix]
        node_cache = {}
        for _, line in self.get_lines():
            if not self.is_node_line(line):
                continue
            node_id, lat, lon = self.get_node_info(line)
            if OpenStreetMapParser.is_in_partition(lon, lat, partition):
                node_cache[node_id] = (lat, lon)

        return node_cache
