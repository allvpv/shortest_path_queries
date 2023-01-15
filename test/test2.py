import argparse
import grpc
import subprocess
import sys
import time
import os

import executer_pb2
import executer_pb2_grpc

def create_logfile(logfile_name):
    print(f"Creating logfile: {logfile_name}")
    return open(logfile_name, "w")

def get_query(stub, node_from, node_to):
    print("Trying to get shortest path from {} to {} ...".format(node_from, node_to))
    resp = stub.ShortestPathQuery(executer_pb2.QueryData(node_id_from=node_from, node_id_to=node_to))
    print(f"Shortest path length received: {resp.shortest_path_len}")

def spawn_partitioner(graph_filepath, listening_p, n_partitions, log_file):
    args = [sys.executable, "../partitioner/main.py",
            graph_filepath,
            "--port", f"{listening_p}",
            "--n_partitions", str(n_partitions)]

    return subprocess.Popen(args, stdout = log_file, stderr = log_file)

def spawn_executer(listening_p, manager_p, log_file):
    args = ["bazel", "run",
            "//executer", "--",
            "--listening-addr", f"[::1]:{listening_p}",
            "--manager-addr", f"http://[::1]:{manager_p}"]

    env = dict(os.environ, RUST_LOG='executer=debug')
    return subprocess.Popen(args, env = env, stdout = log_file, stderr = log_file)

def spawn_worker(listening_p, manager_p, log_file):
    args = ["bazel", "run",
            "//worker", "--",
            "--listening-addr", f"[::1]:{listening_p}",
            "--manager-addr", f"http://[::1]:{manager_p}"]

    env = dict(os.environ, RUST_LOG='worker=debug')
    return subprocess.Popen(args, env = env, stdout = log_file, stderr = log_file)

def terminate(processes):
    for process in processes:
        try:
            process.terminate()
        except:
            pass

def main():
    parser = argparse.ArgumentParser(description='Run process')
    parser.add_argument("--n-partitions", type=int, help="Run `n` workers",required=True)
    parser.add_argument("--manager-port", type=int, help="Manager listening port",required=True)
    parser.add_argument("--worker-ports", type=int, help="Worker listening ports [k, k+n)",required=True)
    parser.add_argument("--executer-port", type=int, help="Executer listening port",required=True)
    parser.add_argument("--graph-file", type=str, help="Filepath to graph",required=True)
    args = parser.parse_args()

    #
    # Rebuild
    #
    print("(Re)building executer and worker")
    subprocess.run(["bazel", "build", "//executer"])
    subprocess.run(["bazel", "build", "//worker"])
    print("Building finished")

    #
    # Create log files
    #
    log_partitioner = create_logfile("log_partitioner")
    log_executer = create_logfile("log_executer")
    log_workers = []
    for i in range(args.n_partitions):
        log_workers.append(create_logfile(f"log_worker_{i}"))

    processes = []

    #
    # Spawning processes
    #
    try:
        print("Spawning partitioner")
        partitioner = spawn_partitioner(args.graph_file,
                                        args.manager_port,
                                        args.n_partitions,
                                        log_partitioner)
        processes.append(partitioner)

        print("Waiting 2 seconds before spawning workers (graph must be processed)")
        time.sleep(2)

        print("Spawning workers")
        for i in range(args.n_partitions):
            worker = spawn_worker(args.worker_ports + i, args.manager_port, log_workers[i])
            processes.append(worker)

        print("Waiting 5 second before spawning executer (graph must be sent)")
        time.sleep(5)


        print("Spawning executer")
        executer = spawn_executer(args.executer_port, args.manager_port, log_executer)
        processes.append(executer)

        #
        # Connecting to executer
        #
        print("Connecting to executer")
        channel = grpc.insecure_channel(f"[::1]:{args.executer_port}")
        stub = executer_pb2_grpc.ExecuterStub(channel)

        print("Waiting 2 seconds")
        time.sleep(2)

        print("Checking if all processes are still alive")
        for process in processes:
            if process.poll() != None:
                print("Some process died")
                terminate(processes)
                exit(-1)

        #
        # Reading requests from stdin
        #
        print("Receiving commands")
        for line in sys.stdin:
            if 'quit' == line.strip():
                break
            elif line.startswith('spq'):
                args = list(filter(None, line.split(' ')))

                if len(args) != 3:
                    print("bad arguments; two required: `node_from`, `node_to`")
                    continue

                node_from = 0
                node_to = 0

                try:
                    node_from = int(args[1])
                    node_to = int(args[2])
                except ValueError:
                    print("arguments for spq must be integers")

                get_query(stub, node_from, node_to)
    except:
        terminate(processes)
        raise

    terminate(processes)
    exit(0)

if __name__ == "__main__":
    main()
