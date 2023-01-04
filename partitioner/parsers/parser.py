from typing import Text, Tuple, List, Dict

class GraphParser:
    def get_lines(self) -> Tuple[int, Text]:
        pass

    def is_node_line(self, line: Text) -> bool:
        pass

    def get_node_info(self, line: Text) -> Tuple[int, float, float]:
        pass

    def get_partition_nodes(self, partitions: List[List[Tuple[float]]], partition_ix: int) -> Dict[int, Tuple[float, float]]:
        pass

    def get_edge_info(self, line: Text) -> Tuple[int, int, float]:
        pass
