# ---------------------------------------------------------------------------
# Input cache — remembers values between runs
# ---------------------------------------------------------------------------

cache_get() {
  local key="$1"
  local cache_file="${CACHE_DIR}/${ENV_NAME:-_global}"
  if [ -f "$cache_file" ]; then
    grep "^${key}=" "$cache_file" 2>/dev/null | head -1 | cut -d'=' -f2-
  fi
}

cache_set() {
  local key="$1"
  local value="$2"
  local cache_file="${CACHE_DIR}/${ENV_NAME:-_global}"
  mkdir -p "$CACHE_DIR"
  # Remove old entry, append new
  if [ -f "$cache_file" ]; then
    grep -v "^${key}=" "$cache_file" > "${cache_file}.tmp" 2>/dev/null || true
    mv "${cache_file}.tmp" "$cache_file"
  fi
  echo "${key}=${value}" >> "$cache_file"
}

# ---------------------------------------------------------------------------
# Interactive prompt helpers
# ---------------------------------------------------------------------------

prompt_default() {
  local prompt="$1"
  local default="${2:-}"
  local result

  if [ -n "$default" ]; then
    read -rp "$(echo -e "${CYAN}[?]${NC} ${prompt} [${default}]: ")" result
    echo "${result:-$default}"
  else
    read -rp "$(echo -e "${CYAN}[?]${NC} ${prompt}: ")" result
    echo "$result"
  fi
}

# Prompt with cache — checks cache for previous value, saves result
prompt_cached() {
  local key="$1"
  local prompt="$2"
  local fallback="${3:-}"
  local cached
  cached=$(cache_get "$key")
  local default="${cached:-$fallback}"
  local result
  result=$(prompt_default "$prompt" "$default")
  if [ -n "$result" ]; then
    cache_set "$key" "$result"
  fi
  echo "$result"
}

prompt_required() {
  local prompt="$1"
  local result=""
  while [ -z "$result" ]; do
    result=$(prompt_default "$prompt" "${2:-}")
    if [ -z "$result" ]; then
      echo -e "${RED}  This field is required${NC}" >&2
    fi
  done
  echo "$result"
}

# Prompt required with cache
prompt_required_cached() {
  local key="$1"
  local prompt="$2"
  local fallback="${3:-}"
  local cached
  cached=$(cache_get "$key")
  local default="${cached:-$fallback}"
  local result=""
  while [ -z "$result" ]; do
    result=$(prompt_default "$prompt" "$default")
    if [ -z "$result" ]; then
      echo -e "${RED}  This field is required${NC}" >&2
    fi
  done
  cache_set "$key" "$result"
  echo "$result"
}

confirm() {
  local result
  read -rp "$(echo -e "${CYAN}[?]${NC} ${1} (y/n): ")" result
  [ "$result" = "y" ] || [ "$result" = "Y" ]
}

# Show numbered menu, sets MENU_RESULT to selected number (1-based)
# Usage: pick_option "Header" "option1" "option2" ...
MENU_RESULT=0
pick_option() {
  local header="$1"
  shift
  local options=("$@")

  echo ""
  echo -e "${BOLD}${header}${NC}"
  echo ""
  for i in "${!options[@]}"; do
    echo -e "  ${CYAN}$((i + 1))${NC}  ${options[$i]}"
  done
  echo ""

  local choice
  while true; do
    read -rp "$(echo -e "${CYAN}[?]${NC} Select (1-${#options[@]}): ")" choice
    if [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le "${#options[@]}" ]; then
      MENU_RESULT="$choice"
      return 0
    fi
    echo -e "${RED}  Invalid choice${NC}"
  done
}

# List remotes and let user pick one, or create new
# Sets ENV_NAME and calls parse_remote
pick_environment() {
  local remotes=()
  local remote_names=()

  if [ -d "$REMOTES_DIR" ] && [ "$(ls -A "$REMOTES_DIR" 2>/dev/null)" ]; then
    for f in "$REMOTES_DIR"/*; do
      [ -f "$f" ] || continue
      local name=$(basename "$f")
      local content=$(cat "$f" | tr -d '[:space:]')
      remote_names+=("$name")
      remotes+=("${name}  →  ${content}")
    done
  fi

  if [ ${#remotes[@]} -eq 0 ]; then
    warn "No environments configured yet."
    echo ""
    info "Let's set one up."
    COMMAND="init"
    cmd_init
    exit 0
  fi

  # Add "create new" option
  remotes+=("Create new environment")

  pick_option "Select environment" "${remotes[@]}"
  local selected=$?

  if [ "$selected" -gt "${#remote_names[@]}" ]; then
    # Selected "Create new"
    COMMAND="init"
    cmd_init
    exit 0
  fi

  ENV_NAME="${remote_names[$((selected - 1))]}"
  parse_remote
}
