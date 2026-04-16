# ---------------------------------------------------------------------------
# Paths + project config defaults. SCRIPT_DIR honours the DEPLOY_PROJECT_ROOT
# override so Bishop can spawn this script against a project directory that
# differs from where the script itself lives.
# ---------------------------------------------------------------------------

SCRIPT_DIR="${DEPLOY_PROJECT_ROOT:-$(cd "$(dirname "$0")" && pwd)}"
DEPLOY_DIR="${SCRIPT_DIR}/.deploy/infra"
REMOTES_DIR="${SCRIPT_DIR}/.deploy/remotes"
CACHE_DIR="${SCRIPT_DIR}/.deploy/cache"
CONFIG_FILE="${SCRIPT_DIR}/.deploy/config"

# ---------------------------------------------------------------------------
# Load project config (with defaults)
# ---------------------------------------------------------------------------

HEALTH_PATH="/health"
APP_PORT=3000
HEALTH_MATCH="ok"
EXTRA_CONTAINERS=""
DATA_DIRS=""
AUTO_SECRETS=""
DOMAIN_TEMPLATES=""

if [ -f "$CONFIG_FILE" ]; then
  set -a; source "$CONFIG_FILE"; set +a
fi
