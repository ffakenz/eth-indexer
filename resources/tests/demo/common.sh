#!/usr/bin/env bash
set -euo pipefail

# Load environment variables
if [ -f "$(dirname "$0")/.env" ]; then
  # shellcheck disable=SC1091
  source "$(dirname "$0")/.env"
else
  echo "Missing .env file!"
  exit 1
fi

# === REQUIREMENTS ===
command -v jq >/dev/null 2>&1 || { echo "jq not found"; exit 1; }
command -v cast >/dev/null 2>&1 || { echo "cast not found"; exit 1; }

# === HELPERS ===
function log() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

function deploy() {
    local ABI_PATH=$1
    # contract constructor arguments
    local CONTRACT_ARGS=$2

    local BYTECODE=$(jq -r '.bytecode.object' $ABI_PATH)
    local CONSTRUCTOR_ARGS=$(cast abi-encode "constructor(address)" $CONTRACT_ARGS | sed 's/^0x//')

    cast send \
        --rpc-url $RPC_URL \
        --private-key $PK_OWNER \
        --create $BYTECODE$CONSTRUCTOR_ARGS \
        --json \
        $CONTRACT_ARGS \
        -- --gas-limit $GAST_LIMIT
}

function send_tx() {
    local CONTRACT_ADDR=$1
    local PK=$2
    
    # remove both $CONTRACT_ADDR and $PK from $@
    # so they don’t sneak into the ABI call.
    shift 2
    
    cast send "$CONTRACT_ADDR" "$@" \
      --rpc-url $RPC_URL \
      --private-key $PK \
      --gas-limit $GAST_LIMIT
}

function balance_of() {
    local CONTRACT_ADDR=$1
    local ACCOUNT=$2
    cast call $1 "balanceOf(address)(uint256)" $2 --rpc-url $RPC_URL
}
