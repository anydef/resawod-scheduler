output "stack_id" {
  description = "ID of the deployed Portainer stack"
  value       = portainer_stack.resawod-scheduler.id
}

output "stack_name" {
  description = "Name of the deployed stack"
  value       = portainer_stack.resawod-scheduler.name
}

output "access_url" {
  description = "URL to access the deployed application"
  value       = "http://${var.app_host}:${var.app_port}"
}

output "portainer_stack_url" {
  description = "URL to view the stack in Portainer"
  value       = "${var.portainer_url}/#/stacks/${portainer_stack.resawod-scheduler.id}"
}