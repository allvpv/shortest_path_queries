from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional

DESCRIPTOR: _descriptor.FileDescriptor

class QueryData(_message.Message):
    __slots__ = ["node_id_from", "node_id_to"]
    NODE_ID_FROM_FIELD_NUMBER: _ClassVar[int]
    NODE_ID_TO_FIELD_NUMBER: _ClassVar[int]
    node_id_from: int
    node_id_to: int
    def __init__(self, node_id_from: _Optional[int] = ..., node_id_to: _Optional[int] = ...) -> None: ...

class QueryFinished(_message.Message):
    __slots__ = ["shortest_path_len"]
    SHORTEST_PATH_LEN_FIELD_NUMBER: _ClassVar[int]
    shortest_path_len: int
    def __init__(self, shortest_path_len: _Optional[int] = ...) -> None: ...
