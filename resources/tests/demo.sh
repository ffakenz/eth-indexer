#!/usr/bin/env bash
set -euo pipefail

# === REQUIREMENTS ===
command -v jq >/dev/null 2>&1 || { echo "jq not found"; exit 1; }
command -v cast >/dev/null 2>&1 || { echo "cast not found"; exit 1; }

# === CONFIG ===
RPC_URL="http://127.0.0.1:8545"

# Estimated gas fee
GAST_LIMIT=3000000

# Accounts from Anvil
OWNER="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
ALICE="0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
BOB="0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"

# Private keys from Anvil (for signing)
PK_OWNER="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
PK_ALICE="0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
PK_BOB="0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"

# === HELPERS ===
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

# --- START ---
CONTRACT_PATH="./zamatoken/ZamaToken.json"
echo "Demo Contract: $CONTRACT_PATH"
echo "Using deployer: $OWNER"
echo "Target addresses: $ALICE, $BOB"

# --- STEP 1: DEPLOY CONTRACT ---
# Selected demo contract
DEPLOY=$(deploy $CONTRACT_PATH $OWNER)
CONTRACT_ADDR=$(echo $DEPLOY | jq -r '.contractAddress')
echo "Deployed Contract at $CONTRACT_ADDR"

# --- STEP 2: MINT TOKEN TO ALICE ---
echo "Minting 1000 tokens to alice"
send_tx $CONTRACT_ADDR $PK_OWNER "mint(address,uint256)" $ALICE 1000
echo "Mint done"

# --- STEP 3: TRANSFERS ---
# x: alice -> bob
echo "Transfer 1: alice -> bob (100)"
send_tx $CONTRACT_ADDR $PK_ALICE "transfer(address,uint256)" $BOB 100

# y: alice -> bob
echo "Transfer 2: alice -> bob (50)"
send_tx $CONTRACT_ADDR $PK_ALICE "transfer(address,uint256)" $BOB 50

# z: bob -> alice
echo "Transfer 3: bob -> alice (25)"
send_tx $CONTRACT_ADDR $PK_BOB "transfer(address,uint256)" $ALICE 50

# --- END ---
echo "All transfers done!"
echo "Final balances:"
echo "Alice: $(balance_of $CONTRACT_ADDR $ALICE)"
echo "Bob:   $(balance_of $CONTRACT_ADDR $BOB)"
