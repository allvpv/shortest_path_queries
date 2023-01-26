if (($# > 0)) && [[ $1 =~ ^-?[0-9]+$ ]]; then
  replicas=$1
else
  echo "no argument provided, defaulting to 4 workers"
  replicas=4 # default value
fi
kubectl delete configmap worker-config
kubectl create configmap worker-config --from-literal=WORKER_REPLICAS=$replicas

kubectl apply -f deployments/partitioner.yaml
kubectl apply -f services/partitioner.yaml
kubectl rollout status deployment/partitioner # wait for deployment to finish
sleep 10 # and a safety margin

kubectl apply -f deployments/worker.yaml
kubectl apply -f services/worker.yaml
kubectl scale deployment worker --replicas=$replicas
kubectl rollout status deployment/worker # wait for deployment to finish
sleep 10 # and a safety margin

kubectl apply -f deployments/executor.yaml
kubectl apply -f services/executor.yaml
kubectl rollout status deployment/executor

# only once: allow TCP traffic to executor:49999
#  gcloud compute firewall-rules create allow-executor-tcp   --allow=tcp:49999

echo "use \"kubectl get services\" until the executor EXTERNAL_IP is available"


