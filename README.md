# Graph Worker

* `protos/`

  Our API.

* `worker/`

  This is the working process running on a single node

  ```shell
  bazel run //worker
  ```

* `partitioner/`

   Partitioner/manager node.

    ```shell
    bazel run //partitioner -- --help
    ```

* `executer/`
   Executer node (to be done).


