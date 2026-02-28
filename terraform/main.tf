module "portainer_stack" {
  # source = "github.com/anydef/build-tools//terraform/portainer-stack?ref=main"
  source = "../.build/build-tools/terraform/portainer-stack"

  stack_name         = var.stack_name
  endpoint_id        = var.endpoint_id
  stack_file_content = file("${path.module}/../docker-compose.yml")
  docker_registry    = var.docker_registry
  force_update       = var.force_update
}
