resource "portainer_stack" "resawod-scheduler" {
  name        = var.stack_name
  endpoint_id = var.endpoint_id

  # Deployment type for Docker Standalone (not Swarm or Kubernetes)
  deployment_type = "standalone"

  # Method: string (inline content)
  method = "string"

  # Docker compose file content as string
  stack_file_content = file("${path.module}/../docker-compose.yml")

  # Environment variables to pass to the stack
  env {
    name  = "DOCKER_REGISTRY"
    value = var.docker_registry
  }

  env {
    name  = "FORCE_UPDATE"
    value = var.force_update != "" ? var.force_update : "none"
  }
}