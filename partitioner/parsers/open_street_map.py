from parsers.parser import GraphParser
from typing import Text, Tuple
import re
import gzip

class OpenStreetMapParser(GraphParser):
    def __init__(self, graph_path: Text):
        self.graph_path = graph_path
        self.node_regex = re.compile(r'\s<node.*')
        self.id_regex = re.compile(r'id="(\d+)"')
        self.lat_regex = re.compile(f'lat="([\d\.]+)"')
        self.lon_regex = re.compile(f'lon="([\d\.]+)"')

    def get_lines(self) -> Tuple[int, Text]:
        with gzip.open(self.graph_path, "r") as f:
            for ix, line in enumerate(f):
                yield ix, line.decode('utf-8')

    def is_node_line(self, line: Text) -> bool:
        return self.node_regex.match(line) != None

    def get_node_info(self, line: Text) -> Tuple[int, float, float]:
        id_match = self.id_regex.search(line)
        lat_match = self.lat_regex.search(line)
        lon_match = self.lon_regex.search(line)

        return int(id_match.group(1)), float(lat_match.group(1)), float(lon_match.group(1))

    def get_edge_info(self, line: Text) -> Tuple[int, int, float]:
        pass
