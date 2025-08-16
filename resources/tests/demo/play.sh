#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

# Load environment variables
ENV_FILE="$(dirname "$0")/.dev-env"
if [ -f "$ENV_FILE" ]; then
  # shellcheck disable=SC1091
  source "$ENV_FILE"
else
  echo "Missing .dev-env file!"
  exit 1
fi

# --- TRANSFERS ---
# alice -> bob
echo "Transfer 1: alice -> bob (100)"
send_tx "$CONTRACT_ADDR" "$PK_ALICE" "transfer(address,uint256)" "$BOB" 100

# alice -> bob
echo "Transfer 2: alice -> bob (50)"
send_tx "$CONTRACT_ADDR" "$PK_ALICE" "transfer(address,uint256)" "$BOB" 50

# bob -> alice
echo "Transfer 3: bob -> alice (25)"
send_tx "$CONTRACT_ADDR" "$PK_BOB" "transfer(address,uint256)" "$ALICE" 50

# --- END ---
echo "All transfers done!"
echo "Final balances:"
echo "Alice: $(balance_of "$CONTRACT_ADDR" "$ALICE")"
echo "Bob:   $(balance_of "$CONTRACT_ADDR" "$BOB")"
