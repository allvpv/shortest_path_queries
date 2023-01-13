pub mod manager {
    tonic::include_proto!("manager");
}

pub mod worker {
    tonic::include_proto!("worker");
}

pub mod executer {
    tonic::include_proto!("executer");
}
