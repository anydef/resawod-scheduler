variable "portainer_url" {
  description = "Portainer instance URL"
  type        = string
  default     = "http://192.168.1.234:9000"
}

variable "portainer_api_key" {
  description = "Portainer API access token"
  type        = string
  sensitive   = true
  # Set via environment variable TF_VAR_portainer_api_key
  # or use terraform.tfvars file (not committed to git)
}

variable "docker_registry" {
  description = "Docker registry address"
  type        = string
  default     = "http://192.168.1.234:5050"
}

variable "stack_name" {
  description = "Name of the Portainer stack"
  type        = string
  default     = "resawod-scheduler"
}

variable "endpoint_id" {
  description = "Portainer endpoint ID (check Portainer UI or API for correct ID)"
  type        = number
  default     = 3
}

variable "force_update" {
  description = "Set to a new value (e.g., timestamp) to force stack recreation"
  type        = string
  default     = ""
}

variable "app_host" {
  description = "Host where the application is deployed"
  type        = string
  default     = "192.168.1.234"
}

variable "app_port" {
  description = "Port the application listens on"
  type        = number
  default     = 3009
}