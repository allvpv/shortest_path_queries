from google.protobuf import empty_pb2 as _empty_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Edge(_message.Message):
    __slots__ = ["node_from_id", "node_to_id", "node_to_worker_id", "weight"]
    NODE_FROM_ID_FIELD_NUMBER: _ClassVar[int]
    NODE_TO_ID_FIELD_NUMBER: _ClassVar[int]
    NODE_TO_WORKER_ID_FIELD_NUMBER: _ClassVar[int]
    WEIGHT_FIELD_NUMBER: _ClassVar[int]
    node_from_id: int
    node_to_id: int
    node_to_worker_id: int
    weight: int
    def __init__(self, node_from_id: _Optional[int] = ..., node_to_id: _Optional[int] = ..., weight: _Optional[int] = ..., node_to_worker_id: _Optional[int] = ...) -> None: ...

class GraphPiece(_message.Message):
    __slots__ = ["edges", "nodes"]
    EDGES_FIELD_NUMBER: _ClassVar[int]
    NODES_FIELD_NUMBER: _ClassVar[int]
    edges: Edge
    nodes: Node
    def __init__(self, nodes: _Optional[_Union[Node, _Mapping]] = ..., edges: _Optional[_Union[Edge, _Mapping]] = ...) -> None: ...

class Node(_message.Message):
    __slots__ = ["node_id"]
    NODE_ID_FIELD_NUMBER: _ClassVar[int]
    node_id: int
    def __init__(self, node_id: _Optional[int] = ...) -> None: ...

class WorkerMetadata(_message.Message):
    __slots__ = ["worker_id"]
    WORKER_ID_FIELD_NUMBER: _ClassVar[int]
    worker_id: int
    def __init__(self, worker_id: _Optional[int] = ...) -> None: ...

class WorkerProperties(_message.Message):
    __slots__ = ["listening_address"]
    LISTENING_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    listening_address: str
    def __init__(self, listening_address: _Optional[str] = ...) -> None: ...

class WorkersList(_message.Message):
    __slots__ = ["workers"]
    class WorkerEntry(_message.Message):
        __slots__ = ["address", "worker_id"]
        ADDRESS_FIELD_NUMBER: _ClassVar[int]
        WORKER_ID_FIELD_NUMBER: _ClassVar[int]
        address: str
        worker_id: int
        def __init__(self, worker_id: _Optional[int] = ..., address: _Optional[str] = ...) -> None: ...
    WORKERS_FIELD_NUMBER: _ClassVar[int]
    workers: _containers.RepeatedCompositeFieldContainer[WorkersList.WorkerEntry]
    def __init__(self, workers: _Optional[_Iterable[_Union[WorkersList.WorkerEntry, _Mapping]]] = ...) -> None: ...
