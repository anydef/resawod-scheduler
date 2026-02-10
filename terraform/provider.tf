terraform {
  required_version = ">= 1.0"

  required_providers {
    portainer = {
      source  = "portainer/portainer"
      version = "~> 1.0"
    }
  }
}

provider "portainer" {
  endpoint = var.portainer_url
  api_key  = var.portainer_api_key
}