#!/bin/bash

set -e

# =============================================================================
# Portainer Stack Deployment via Terraform
# =============================================================================
# This script wraps Terraform deployment, loading secrets from .env and
# passing them as Terraform variables.
#
# The legacy API-based deployment script is available as:
# deploy-portainer-legacy.sh
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="${SCRIPT_DIR}/terraform"

echo "========================================"
echo "Portainer Stack Deployment (Terraform)"
echo "========================================"

# Load environment variables from .env
if [ -f "$SCRIPT_DIR/.env" ]; then
    echo "Loading environment from .env..."
    source "$SCRIPT_DIR/.env"
else
    echo "Error: .env file not found in $SCRIPT_DIR"
    exit 1
fi

# Validate required variables
if [ -z "$PORTAINER_URL" ] || [ -z "$PORTAINER_ACCESS_TOKEN" ] || [ -z "$DOCKER_REGISTRY" ]; then
    echo "Error: Missing required environment variables in .env"
    echo "Required: PORTAINER_URL, PORTAINER_ACCESS_TOKEN, DOCKER_REGISTRY"
    exit 1
fi

# Export Terraform variables (TF_VAR_* are automatically picked up by Terraform)
export TF_VAR_portainer_url="${PORTAINER_URL%/}"  # Remove trailing slash
export TF_VAR_portainer_api_key="${PORTAINER_ACCESS_TOKEN}"
export TF_VAR_docker_registry="${DOCKER_REGISTRY}"
export TF_VAR_force_update="$(date +%s)"  # Timestamp to force stack update

echo "Configuration:"
echo "  Portainer URL: ${TF_VAR_portainer_url}"
echo "  Docker Registry: ${TF_VAR_docker_registry}"
echo "  Terraform Directory: ${TERRAFORM_DIR}"
echo "========================================"

# Check if Terraform is installed
if ! command -v terraform &> /dev/null; then
    echo "Error: terraform is not installed or not in PATH"
    echo "Install from: https://www.terraform.io/downloads"
    exit 1
fi

# Initialize Terraform if needed
if [ ! -d "${TERRAFORM_DIR}/.terraform" ]; then
    echo "Initializing Terraform..."
    terraform -chdir="${TERRAFORM_DIR}" init
    echo ""
fi

# Plan the deployment
echo "Planning deployment..."
terraform -chdir="${TERRAFORM_DIR}" plan
echo ""

# Ask for confirmation before applying
read -p "Apply this configuration? (yes/no): " confirm
if [ "$confirm" != "yes" ] && [ "$confirm" != "y" ]; then
    echo "Deployment cancelled."
    exit 0
fi

# Apply the configuration
echo ""
echo "Applying Terraform configuration..."
terraform -chdir="${TERRAFORM_DIR}" apply -auto-approve

# Display outputs
echo ""
echo "========================================"
echo "✓ Deployment completed successfully!"
echo "========================================"
echo ""

# Get outputs from Terraform
terraform -chdir="${TERRAFORM_DIR}" output

echo ""
echo "Next steps:"
echo "  - Access the application at the URL above"
echo "  - View the stack in Portainer UI"
echo "  - Check logs: docker logs helloworld-python"
echo ""
