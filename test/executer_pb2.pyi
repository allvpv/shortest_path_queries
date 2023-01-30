from google.protobuf import empty_pb2 as _empty_pb2
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional

DESCRIPTOR: _descriptor.FileDescriptor

class CoordinateResponse(_message.Message):
    __slots__ = ["lat", "lon"]
    LAT_FIELD_NUMBER: _ClassVar[int]
    LON_FIELD_NUMBER: _ClassVar[int]
    lat: float
    lon: float
    def __init__(self, lat: _Optional[float] = ..., lon: _Optional[float] = ...) -> None: ...

class Node(_message.Message):
    __slots__ = ["node_id", "worker_id"]
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    WORKER_ID_FIELD_NUMBER: _ClassVar[int]
    node_id: int
    worker_id: int
    def __init__(self, node_id: _Optional[int] = ..., worker_id: _Optional[int] = ...) -> None: ...

class NodeCoordinates(_message.Message):
    __slots__ = ["node_id", "query_id", "worker_id"]
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    QUERY_ID_FIELD_NUMBER: _ClassVar[int]
    WORKER_ID_FIELD_NUMBER: _ClassVar[int]
    node_id: int
    query_id: int
    worker_id: int
    def __init__(self, node_id: _Optional[int] = ..., worker_id: _Optional[int] = ..., query_id: _Optional[int] = ...) -> None: ...

class QueryData(_message.Message):
    __slots__ = ["node_id_from", "node_id_to"]
    NODE_ID_FROM_FIELD_NUMBER: _ClassVar[int]
    NODE_ID_TO_FIELD_NUMBER: _ClassVar[int]
    node_id_from: int
    node_id_to: int
    def __init__(self, node_id_from: _Optional[int] = ..., node_id_to: _Optional[int] = ...) -> None: ...

class QueryId(_message.Message):
    __slots__ = ["query_id"]
    QUERY_ID_FIELD_NUMBER: _ClassVar[int]
    query_id: int
    def __init__(self, query_id: _Optional[int] = ...) -> None: ...

class QueryResults(_message.Message):
    __slots__ = ["query_id", "shortest_path_len"]
    QUERY_ID_FIELD_NUMBER: _ClassVar[int]
    SHORTEST_PATH_LEN_FIELD_NUMBER: _ClassVar[int]
    query_id: int
    shortest_path_len: int
    def __init__(self, query_id: _Optional[int] = ..., shortest_path_len: _Optional[int] = ...) -> None: ...
