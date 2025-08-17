## 🎮 Demo

For a quick test, the repo includes a local demo workflow with a **toy ERC-20 contract** (created using [Foundry Forge](https://getfoundry.sh/forge/overview/)) and **transfers between Alice & Bob** while the engine is running in the background.

### Step 1: Start a local devnet fork

> We used [Anvil](https://getfoundry.sh/anvil/overview) for this.

```sh
anvil --block-time 5
```

### Step 2: Configure environment
Edit the [.env](../tests/demo/.env) file.

> The existing file contains sample default entries from local Anvil.

### Step 3: Run the end-to-end demo

```sh
make demo.test
```
