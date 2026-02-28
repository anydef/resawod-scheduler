# =============================================================================
# resawod-scheduler
# =============================================================================
# Required: a .env or .env.tpl file with at minimum DOCKER_REGISTRY set.
# See config.toml.example for application configuration.
# =============================================================================

DOCKER_IMAGE_NAME := resawod-scheduler
BASE_IMAGE_NAME   := resawod-base
IMAGE_TAG         := latest
BUILD_CONTEXT     := $(CURDIR)
TERRAFORM_DIR     := $(CURDIR)/terraform

BUILD_TOOLS_DIR := .build/build-tools

# Uncomment following for local testing.
#BUILD_TOOLS_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))../build-tools)

-include $(BUILD_TOOLS_DIR)/common.mk
$(BUILD_TOOLS_DIR)/common.mk:
	git clone --depth=1 https://github.com/anydef/build-tools $(BUILD_TOOLS_DIR)
