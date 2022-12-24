# Graph Worker

* `protos/`

  Our API.

* `worker/`

  This is the working process running on a single node

  ```shell
  cd worker/
  cargo run -- --help
  ```

* `partitioner/`

   Partitioner/manager node.

    ```shell
    bazel run //partitioner:partitioner -- --help
    ```

* `executer/`
   Executer node (to be done).


