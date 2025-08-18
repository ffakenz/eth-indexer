
## 🌈 Features

- **Indexer**
  Ingests filtered on-chain logs from RPC (source) and persists transfer events in SQLite (sink).
  - **Gapfiller** → (async) on startup, it backfills log events in batches from a given block number
  - **Live-Watcher** → (async) streams incoming logs for processing
  - **Block bloom filtering** → used by both Gapfiller (`eth_getLogs`) and Live-Watcher (`eth_getFilterChanges`) under the hood to efficiently skip blocks without relevant events.
  - **Checkpointer** → (periodically) persists checkpoint snapshots at a configurable interval

- **CLI**
  - `engine` → start indexing
  - `select` → query stored index data (output JSON)

- **Data Model**
  - *Data integrity:* uses `(tx hash + log index)` as unique identifier
  - *Deduplication:* uses `INSERT OR IGNORE` to gracefully skip UNIQUE constraint errors
  - *Efficient data storage:* data model optimized for both **performance** and **space**

- **Flexible**
  - Storage via [SQLx](https://github.com/launchbadge/sqlx) → async, compile-time checked queries, runtime and database agnostic
  - Ethereum-Toolkit via [Alloy](https://alloy.rs/introduction/why-alloy) → modular, high-performance, and developer-friendly experience for building on EVM-compatible chains

### Later

- [ ] Handling reorgs and finality

- [ ] Expose engine prometheus metrics via http server
