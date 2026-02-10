output "stack_id" {
  description = "ID of the deployed Portainer stack"
  value       = portainer_stack.video-transcoder.id
}

output "stack_name" {
  description = "Name of the deployed stack"
  value       = portainer_stack.video-transcoder.name
}

output "access_url" {
  description = "URL to access the deployed application"
  value       = "http://192.168.1.234:8078"
}

output "portainer_stack_url" {
  description = "URL to view the stack in Portainer"
  value       = "${var.portainer_url}/#/stacks/${portainer_stack.video-transcoder.id}"
}