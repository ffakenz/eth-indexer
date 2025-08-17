## 📖 Usage

Reference
```sh
eth-indexer --help
```

### 1. Start the Engine

Run the indexer to watch for transfer events

```sh
# options:
# --rpc-url             → evm-node JSON-RPC endpoint
# --db-url              → SQLite connection string
# --signer-pk           → user signing private key
# --addresses           → contract(s) to index
# --event               → supported event types (`transfer`)
# --checkpoint-interval → snapshot frequency
# --poll-interval       → node polling interval (ms)
eth-indexer engine \
    --rpc-url "$RPC_URL" \
    --db-url "sqlite:$DB_FILE" \
    --signer-pk "$PK" \
    --addresses "$CONTRACT_ADDR" \
    --event transfer \
    --from-block "$BLOCK_NBR" \
    --checkpoint-interval 12 \
    --poll-interval 500
```

### 2. Query Stored Data

Select last **checkpoint**

```sh
eth-indexer select \
    --db-url "sqlite:$DB_FILE" \
    --entity checkpoint \
    --from-block last
```

Select **transfers** from a given block

```sh
eth-indexer select \
    --db-url "sqlite:$DB_FILE" \
    --entity transfer \
    --from-block "$BLOCK_NBR"
```
