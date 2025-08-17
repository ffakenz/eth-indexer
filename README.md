# Overview

**eth-indexer** is a powerful command-line interface (CLI) tool designed to index filtered ETH log events.

It connects to any *EVM-compatible node* via **JSON-RPC**, scans logs emitted from chosen **ERC-20 contracts**, and efficiently persists **Transfer** events into a local **SQLite** database file.

*eth-indexer is open-source, modular, and designed for easy extension.*

## Quick Start

After [installation](resources/docs/install.md), start the CLI and explore.
```sh
eth-indexer --help
```

> For more detailed information, see the [`docs`](resources/docs) section.

## 🛠️ Development

[![Rust](https://img.shields.io/badge/rust-1.88.0-6b00bc388?logo=rust&logoColor=white)](https://www.rust-lang.org/tools/install)
[![Cargo](https://img.shields.io/badge/cargo-1.88.0-873a06493?logo=rust&logoColor=white)](https://doc.rust-lang.org/cargo/)

Refer to [`Makefile`](Makefile) for common tasks

```sh
make fmt       # format code
make build     # build workspace
make check     # lint + tests
```

## 🤝 Contributing

Contributions are welcome!

🙏 Please open issues or PRs for bug reports, features, or improvements.

Refer to the [CONTRIBUTING.md](CONTRIBUTING.md) file for details on how to contribute to this project.

## 📜 License

This project is licensed under the **MIT License** – see [LICENSE](LICENSE) for details.
