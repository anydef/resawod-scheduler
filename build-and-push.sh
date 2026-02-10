#!/bin/bash

set -e

# Load environment variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -f "$SCRIPT_DIR/.env" ]; then
    source "$SCRIPT_DIR/.env"
else
    echo "Error: .env file not found in $SCRIPT_DIR"
    exit 1
fi

# Variables
IMAGE_NAME="scheduler"
IMAGE_TAG="latest"
BASE_IMAGE_NAME="$IMAGE_NAME-base"
MAIN_IMAGE_NAME="resawod-$IMAGE_NAME"
FULL_IMAGE_NAME="${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================"
echo "Building and Pushing Docker Images"
echo "========================================"
echo -e "Registry: ${YELLOW}${DOCKER_REGISTRY}${NC}"
echo -e "Image: ${YELLOW}${IMAGE_NAME}:${IMAGE_TAG}${NC}"
echo -e "Full name: ${YELLOW}${FULL_IMAGE_NAME}${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Build base image
echo -e "${BLUE}[1/3] Building base image...${NC}"
docker build -f "$SCRIPT_DIR/Dockerfile.base" -t "${BASE_IMAGE_NAME}:${IMAGE_TAG}" "$SCRIPT_DIR"
echo -e "${GREEN}✓ Base image built successfully${NC}"
echo ""

# Build main image
echo -e "${BLUE}[2/3] Building main image...${NC}"
docker build -t "${MAIN_IMAGE_NAME}:${IMAGE_TAG}" "$SCRIPT_DIR"
echo -e "${GREEN}✓ Main image built successfully${NC}"
echo ""

# Tag the image for the registry
echo -e "${BLUE}[3/3] Tagging and pushing to registry...${NC}"
docker tag "${MAIN_IMAGE_NAME}:${IMAGE_TAG}" "${FULL_IMAGE_NAME}"
echo -e "${GREEN}✓ Image tagged for registry${NC}"

# Push to registry
echo "Pushing to ${DOCKER_REGISTRY}..."
docker push "${FULL_IMAGE_NAME}"
echo -e "${GREEN}✓ Image pushed successfully${NC}"
echo ""

echo -e "${GREEN}========================================"
echo "✓ Build and Push Complete"
echo "========================================"
echo -e "Local images:${NC}"
echo "  - ${BASE_IMAGE_NAME}:${IMAGE_TAG}"
echo "  - ${MAIN_IMAGE_NAME}:${IMAGE_TAG}"
echo ""
echo -e "${GREEN}Registry image:${NC}"
echo "  - ${FULL_IMAGE_NAME}"
echo -e "${GREEN}========================================${NC}"