#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

# --- START ---

# Selected demo contract
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTRACT_PATH="$SCRIPT_DIR/../zamatoken/ZamaToken.json"

echo "Demo Contract: $CONTRACT_PATH"
echo "Using deployer: $OWNER"
echo "Target addresses: $ALICE, $BOB"

# --- STEP 1: DEPLOY CONTRACT ---
BLOCK_NBR=$(cast block-number)
echo "Block number $BLOCK_NBR"
DEPLOY=$(deploy "$CONTRACT_PATH" "$OWNER")
CONTRACT_ADDR=$(echo "$DEPLOY" | jq -r '.contractAddress')
echo "Deployed Contract at $CONTRACT_ADDR"

# --- STEP 2: MINT TOKENS TO ALICE ---
echo "Minting 1000 tokens to alice"
send_tx "$CONTRACT_ADDR" "$PK_OWNER" "mint(address,uint256)" "$ALICE" 1000
echo "Mint done"

# --- STEP 3: PREPARE .dev-env ---
ENV_FILE="$(dirname "$0")/.dev-env"
touch "$ENV_FILE"

if grep -q '^CONTRACT_ADDR=' "$ENV_FILE"; then
  # replace existing line
  sed -i '' -e "s|^CONTRACT_ADDR=.*|CONTRACT_ADDR=$CONTRACT_ADDR|" "$ENV_FILE"
else
  # append if not present
  echo "CONTRACT_ADDR=$CONTRACT_ADDR" >> "$ENV_FILE"
fi

if grep -q '^BLOCK_NBR=' "$ENV_FILE"; then
  # replace existing line
  sed -i '' -e"s|^BLOCK_NBR=.*|BLOCK_NBR=$BLOCK_NBR|" "$ENV_FILE"
else
  # append if not present
  echo "BLOCK_NBR=$BLOCK_NBR" >> "$ENV_FILE"
fi
