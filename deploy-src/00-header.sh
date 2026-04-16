#!/bin/bash
# =============================================================================
# Deploy CLI — single entry point for all deployment operations
#
# Usage:
#   ./deploy                       Interactive setup (walks you through everything)
#   ./deploy <environment>         Deploy to environment
#   ./deploy <command> <env>       Run a specific command
#
# Commands:
#   init                              Full interactive setup (server + app + remote)
#   setup-server <env>                Set up shared infrastructure on server
#   setup-app <env>                   Set up app on server (DB, .env, etc.)
#   <env>                             Deploy the app
#   logs <env>                        Tail app logs
#   status <env>                      Show running containers
#   ssh <env>                         Open SSH session to server
#
# Remote format (.deploy/remotes/<env>):
#   user@host:app-name.domain
#
# NOTE: This file is built from deploy-src/ by deploy-src/build.sh.
# Do not edit directly — edit the source fragments and rebuild.
# =============================================================================
set -euo pipefail
