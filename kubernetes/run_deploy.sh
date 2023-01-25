kubectl apply -f deployments/partitioner.yaml
kubectl apply -f services/partitioner.yaml
kubectl rollout status deployment/partitioner # wait for deployment to finish
sleep 5 # and a safety margin

kubectl apply -f deployments/worker.yaml
kubectl apply -f services/worker.yaml
kubectl rollout status deployment/worker # wait for deployment to finish
sleep 5 # and a safety margin

kubectl apply -f deployments/executor.yaml
kubectl apply -f services/executor.yaml
kubectl rollout status deployment/executor

# only once: allow TCP traffic to executor:49999
#  gcloud compute firewall-rules create allow-executor-tcp   --allow=tcp:49999