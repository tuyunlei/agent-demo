#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATES_DIR="$ROOT_DIR/crates"

if [[ ! -d "$CRATES_DIR" ]]; then
  echo "ERROR: crates directory not found: $CRATES_DIR"
  exit 1
fi

# Forbidden dependency directions (inner layer cannot depend on outer layer)
declare -A FORBIDDEN
FORBIDDEN["agent-types"]="agent-app agent-channel agent-server agent-storage agent-llm agent-proto agent-domain"
FORBIDDEN["agent-domain"]="agent-app agent-channel agent-server agent-storage agent-llm agent-proto"
FORBIDDEN["agent-app"]="agent-channel agent-server agent-storage agent-llm agent-proto"
FORBIDDEN["agent-proto"]="agent-domain agent-app agent-channel agent-server agent-storage agent-llm"
FORBIDDEN["agent-storage"]="agent-app agent-channel agent-server agent-llm agent-proto"
FORBIDDEN["agent-llm"]="agent-app agent-channel agent-server agent-storage agent-proto"
FORBIDDEN["agent-channel"]=""
FORBIDDEN["agent-server"]=""

extract_agent_deps() {
  local cargo_toml="$1"
  awk '
    BEGIN { in_deps = 0 }

    /^\[[^]]+\][[:space:]]*$/ {
      in_deps = 0
      if ($0 ~ /^\[(target\.[^]]+\.)?(dev-|build-)?dependencies\][[:space:]]*$/) {
        in_deps = 1
      }
      next
    }

    in_deps {
      line = $0
      sub(/#.*/, "", line)
      if (line ~ /^[[:space:]]*$/) next

      if (match(line, /^[[:space:]]*"?(agent-[A-Za-z0-9_-]+)"?[[:space:]]*=/, m)) {
        print m[1]
      }
    }
  ' "$cargo_toml" | sort -u
}

violations=0
warnings=0

while IFS= read -r cargo_toml; do
  crate="$(basename "$(dirname "$cargo_toml")")"

  if [[ -z "${FORBIDDEN[$crate]+x}" ]]; then
    echo "WARN: No architecture rule configured for crate '$crate'. Skipping strict checks for it."
    warnings=$((warnings + 1))
    continue
  fi

  deps="$(extract_agent_deps "$cargo_toml")"
  if [[ -z "$deps" ]]; then
    continue
  fi

  for dep in $deps; do
    for forbidden_dep in ${FORBIDDEN[$crate]}; do
      if [[ "$dep" == "$forbidden_dep" ]]; then
        echo "ERROR: Architecture violation - '$crate' must not depend on '$dep' ($cargo_toml)"
        violations=$((violations + 1))
      fi
    done
  done

done < <(find "$CRATES_DIR" -mindepth 2 -maxdepth 2 -name Cargo.toml | sort)

if (( warnings > 0 )); then
  echo "WARN: Found $warnings crate(s) without explicit architecture rules."
fi

if (( violations > 0 )); then
  echo "Architecture dependency check failed ❌ ($violations violation(s))"
  exit 1
fi

echo "Architecture dependency check passed ✅"
