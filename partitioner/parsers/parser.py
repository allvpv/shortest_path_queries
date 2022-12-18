from typing import Text, Tuple

class GraphParser:
    def get_lines(self) -> Tuple[int, Text]:
        pass

    def is_node_line(self, line: Text) -> bool:
        pass

    def get_node_info(self, line: Text) -> Tuple[int, float, float]:
        pass

    def get_edge_info(self, line: Text) -> Tuple[int, int, float]:
        pass
