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
IMAGE_TAG="latest"
BASE_IMAGE_NAME="resawod-base"
MAIN_IMAGE_NAME="resawod-scheduler"
FULL_BASE_IMAGE="${DOCKER_REGISTRY}/${BASE_IMAGE_NAME}:${IMAGE_TAG}"
FULL_IMAGE_NAME="${DOCKER_REGISTRY}/${MAIN_IMAGE_NAME}:${IMAGE_TAG}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================"
echo "Building and Pushing Docker Images"
echo "========================================"
echo -e "Registry: ${YELLOW}${DOCKER_REGISTRY}${NC}"
echo -e "Base image: ${YELLOW}${FULL_BASE_IMAGE}${NC}"
echo -e "Main image: ${YELLOW}${FULL_IMAGE_NAME}${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Build and push base image
echo -e "${BLUE}[1/4] Building base image...${NC}"
docker build -f "$SCRIPT_DIR/Dockerfile.base" -t "${FULL_BASE_IMAGE}" "$SCRIPT_DIR"
echo -e "${GREEN}✓ Base image built successfully${NC}"
echo ""

echo -e "${BLUE}[2/4] Pushing base image to registry...${NC}"
docker push "${FULL_BASE_IMAGE}"
echo -e "${GREEN}✓ Base image pushed successfully${NC}"
echo ""

# Build main image (uses base image from registry)
echo -e "${BLUE}[3/4] Building main image...${NC}"
docker build --build-arg DOCKER_REGISTRY="${DOCKER_REGISTRY}" -t "${FULL_IMAGE_NAME}" "$SCRIPT_DIR"
echo -e "${GREEN}✓ Main image built successfully${NC}"
echo ""

# Push main image to registry
echo -e "${BLUE}[4/4] Pushing main image to registry...${NC}"
docker push "${FULL_IMAGE_NAME}"
echo -e "${GREEN}✓ Main image pushed successfully${NC}"
echo ""

echo -e "${GREEN}========================================"
echo "✓ Build and Push Complete"
echo "========================================"
echo -e "Registry images:${NC}"
echo "  - ${FULL_BASE_IMAGE}"
echo "  - ${FULL_IMAGE_NAME}"
echo -e "${GREEN}========================================${NC}"