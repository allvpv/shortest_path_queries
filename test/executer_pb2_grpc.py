# Generated by the gRPC Python protocol compiler plugin. DO NOT EDIT!
"""Client and server classes corresponding to protobuf-defined services."""
import grpc

import executer_pb2 as executer__pb2
from google.protobuf import empty_pb2 as google_dot_protobuf_dot_empty__pb2


class ExecuterStub(object):
    """Interface exported by the executer node and exposed to end user (very simple so far).
    """

    def __init__(self, channel):
        """Constructor.

        Args:
            channel: A grpc.Channel.
        """
        self.ShortestPathQuery = channel.unary_unary(
                '/executer.Executer/ShortestPathQuery',
                request_serializer=executer__pb2.QueryData.SerializeToString,
                response_deserializer=executer__pb2.QueryResults.FromString,
                )
        self.BacktrackPathForQuery = channel.unary_stream(
                '/executer.Executer/BacktrackPathForQuery',
                request_serializer=executer__pb2.QueryId.SerializeToString,
                response_deserializer=executer__pb2.Node.FromString,
                )
        self.ForgetQuery = channel.unary_unary(
                '/executer.Executer/ForgetQuery',
                request_serializer=executer__pb2.QueryId.SerializeToString,
                response_deserializer=google_dot_protobuf_dot_empty__pb2.Empty.FromString,
                )


class ExecuterServicer(object):
    """Interface exported by the executer node and exposed to end user (very simple so far).
    """

    def ShortestPathQuery(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')

    def BacktrackPathForQuery(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')

    def ForgetQuery(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details('Method not implemented!')
        raise NotImplementedError('Method not implemented!')


def add_ExecuterServicer_to_server(servicer, server):
    rpc_method_handlers = {
            'ShortestPathQuery': grpc.unary_unary_rpc_method_handler(
                    servicer.ShortestPathQuery,
                    request_deserializer=executer__pb2.QueryData.FromString,
                    response_serializer=executer__pb2.QueryResults.SerializeToString,
            ),
            'BacktrackPathForQuery': grpc.unary_stream_rpc_method_handler(
                    servicer.BacktrackPathForQuery,
                    request_deserializer=executer__pb2.QueryId.FromString,
                    response_serializer=executer__pb2.Node.SerializeToString,
            ),
            'ForgetQuery': grpc.unary_unary_rpc_method_handler(
                    servicer.ForgetQuery,
                    request_deserializer=executer__pb2.QueryId.FromString,
                    response_serializer=google_dot_protobuf_dot_empty__pb2.Empty.SerializeToString,
            ),
    }
    generic_handler = grpc.method_handlers_generic_handler(
            'executer.Executer', rpc_method_handlers)
    server.add_generic_rpc_handlers((generic_handler,))


 # This class is part of an EXPERIMENTAL API.
class Executer(object):
    """Interface exported by the executer node and exposed to end user (very simple so far).
    """

    @staticmethod
    def ShortestPathQuery(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_unary(request, target, '/executer.Executer/ShortestPathQuery',
            executer__pb2.QueryData.SerializeToString,
            executer__pb2.QueryResults.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)

    @staticmethod
    def BacktrackPathForQuery(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_stream(request, target, '/executer.Executer/BacktrackPathForQuery',
            executer__pb2.QueryId.SerializeToString,
            executer__pb2.Node.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)

    @staticmethod
    def ForgetQuery(request,
            target,
            options=(),
            channel_credentials=None,
            call_credentials=None,
            insecure=False,
            compression=None,
            wait_for_ready=None,
            timeout=None,
            metadata=None):
        return grpc.experimental.unary_unary(request, target, '/executer.Executer/ForgetQuery',
            executer__pb2.QueryId.SerializeToString,
            google_dot_protobuf_dot_empty__pb2.Empty.FromString,
            options, channel_credentials,
            insecure, call_credentials, compression, wait_for_ready, timeout, metadata)
