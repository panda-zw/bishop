# =============================================================================
# Parse remote file into variables
# =============================================================================

parse_remote() {
  REMOTE_FILE="${REMOTES_DIR}/${ENV_NAME}"

  if [ ! -f "$REMOTE_FILE" ]; then
    error "Remote '${ENV_NAME}' not found. Run: ./deploy init"
  fi

  REMOTE_RAW=$(cat "$REMOTE_FILE" | tr -d '[:space:]')

  SSH_PART="${REMOTE_RAW%%:*}"
  APP_PART="${REMOTE_RAW#*:}"

  SSH_USER="${SSH_PART%%@*}"
  SSH_HOST="${SSH_PART#*@}"

  APP_NAME="${APP_PART%%.*}"
  APP_DOMAIN="${APP_PART#*.}"

  if [ "$APP_NAME" = "$APP_PART" ]; then
    APP_DOMAIN=""
  fi

  APP_DIR="/opt/apps/${APP_NAME}"
  DATA_DIR="/mnt/data/apps/${APP_NAME}"
}

# ---------------------------------------------------------------------------
# SSH helpers
# ---------------------------------------------------------------------------

remote() {
  ssh -q -o ConnectTimeout=10 "${SSH_USER}@${SSH_HOST}" "$@"
}

remote_check() {
  ssh -q -o ConnectTimeout=5 "${SSH_USER}@${SSH_HOST}" "echo ok" > /dev/null 2>&1 \
    || error "Cannot SSH to ${SSH_USER}@${SSH_HOST}. Check your SSH key and server."
}
