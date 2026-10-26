# Stellar CLI (stellar-cli)

[![Apache 2.0 licensed](https://img.shields.io/badge/license-apache%202.0-blue.svg)](LICENSE) [![Crates.io Version](https://img.shields.io/crates/v/stellar-cli?label=version&color=04ac5b)](https://crates.io/crates/stellar-cli) [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/stellar/stellar-cli)

This repo is home to the Stellar CLI, the command-line multi-tool for running and deploying Stellar contracts on the Stellar network.

## Table of Contents

- [Documentation](#documentation)
- [Cookbook](#cookbook)
- [Install](#install)
- [Autocomplete](#autocomplete)
- [Security & RPC Trust Boundary](#security--rpc-trust-boundary)
- [Latest Release](#latest-release)
- [Upcoming Features](#upcoming-features)
- [To Contribute](#to-contribute)
- [Additional Developer Resources](#additional-developer-resources)

## Documentation

For installation options see below, for usage instructions [see the full help docs](FULL_HELP_DOCS.md).

## Cookbook

To understand how to get the most of the Stellar CLI, see the [Stellar CLI Cookbook](https://github.com/stellar/stellar-cli/tree/main/cookbook) for recipes and a collection of resources to teach you how to use the CLI. Examples of recipes included in the CLI cookbook include: send payments, manage contract lifecycle, extend contract instance/storage/wasm, and more.

## Install

Install with the install script (macOS, Linux):

```sh
curl -fsSL https://github.com/stellar/stellar-cli/raw/main/install.sh | sh
```

For installation options, run the script with `--help`:

```sh
curl -fsSL https://github.com/stellar/stellar-cli/raw/main/install.sh | sh -s -- --help
```

Install with dependency setup (`--install-deps`) to attempt installing:

- Linux development dependencies (`libdbus`/`libudev` development packages; Linux only, not required on macOS)
- Rust toolchain and `wasm32v1-none` target (for contract build workflows)

```sh
curl -fsSL https://github.com/stellar/stellar-cli/raw/main/install.sh | sh -s -- --install-deps
```

Install with Homebrew (macOS, Linux):

```sh
brew install stellar-cli
```

Install the latest version from source:

```sh
cargo install --locked stellar-cli
```

Install without features that depend on additional libraries:

```sh
cargo install --locked stellar-cli --no-default-features
```

Install or run the unreleased main branch with nix:

```console
$ nix run 'github:stellar/stellar-cli' -- --help
or install
$ nix profile install github:stellar/stellar-cli
```

For additional information on how to install, see instructions here on the [Developer Docs](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup#install).

Use GitHub Action:

```yaml
uses: stellar/stellar-cli@v23.0.1
```

Run with Docker:

```sh
docker run --rm -it -v "$(pwd)":/source stellar/stellar-cli version
```

Docker image distribution is handled by the [stellar/stellar-cli-docker](https://github.com/stellar/stellar-cli-docker) repository, with images published to [Docker Hub](https://hub.docker.com/r/stellar/stellar-cli/).

## Autocomplete

The Stellar CLI supports some autocompletion. To set up, run the following commands:

```sh
stellar completion --shell <SHELL>
```

Possible SHELL values are `bash`, `elvish`, `fish`, `powershell`, `zsh`, etc.

To enable autocomplete in the current bash shell, run:

```bash
source <(stellar completion --shell bash)
```

To enable autocomplete permanently, run:

```bash
echo "source <(stellar completion --shell bash)" >> ~/.bashrc
```

## Security & RPC Trust Boundary

The Stellar CLI treats the configured Soroban RPC endpoint (`--rpc-url` / `STELLAR_RPC_URL` or the selected `--network` preset) as a **trusted component**:

- **Transaction simulation & authorization entries**: When invoking or deploying contracts, the CLI calls `simulateTransaction` on the RPC endpoint and constructs the transaction footprint, resource fee, and Soroban authorization entries (`SorobanAuthorizedEntry`) from the simulation response before signing.
- **Signature expiration window**: When signing Soroban authorization entries, the CLI queries `getLatestLedger` from the RPC endpoint and derives `signature_expiration_ledger` from the reported ledger sequence (`latest_ledger + 60`).
- **Account sequence numbers**: When assembling transactions, the CLI fetches the source account's current sequence number from the RPC endpoint.

### Implications of Untrusted or Plain-HTTP Endpoints

Pointing the CLI at an untrusted, compromised, or unencrypted (`http://`) RPC endpoint allows the endpoint operator or an on-path attacker to:

1. **Manipulate simulated authorization payloads**: Return crafted contract invocations, arguments, or sub-invocations in `simulateTransaction` that may be signed if `--auto-sign` is used or if prompts are not carefully inspected.
2. **Skew signature expiration windows**: Report an artificially high or stale ledger sequence in `getLatestLedger`, extending or invalidating the `signature_expiration_ledger` validity window of signed authorization entries.
3. **Observe or tamper with traffic in transit**: Plain-HTTP connections expose transaction payloads, account addresses, and simulation results to network eavesdropping and man-in-the-middle modification.

### Recommendations

- **Use trusted RPC endpoints over TLS (`https://`)**: Always connect to RPC endpoints you operate or explicitly trust, and prefer `https://` URLs with valid TLS certificates for Testnet, Futurenet, and Mainnet operations (reserve `http://` strictly for local development containers on `localhost` / `127.0.0.1`).
- **Review authorization prompts before signing**: Avoid passing `--auto-sign` when interacting with non-local networks, and verify the contract address, function name, arguments, and expiration ledger before approving hardware wallet or local key signatures.

## Latest Release

For the latest release, see [releases](https://github.com/stellar/stellar-cli/releases).

## Upcoming Features

For upcoming features, please see the [project board](https://github.com/orgs/stellar/projects/50).

## To Contribute

Find issues to contribute to [here](https://github.com/stellar/stellar-cli/contribute) and review [CONTRIBUTING.md](/CONTRIBUTING.md).

## Additional Developer Resources

- Developer Docs CLI Examples: https://developers.stellar.org/docs/smart-contracts/guides/cli
- Video Tutorial on `network container`, `keys`, and `contract init`: https://developers.stellar.org/meetings/2024/06/27
- Video Tutorial on `alias` and `snapshot`: https://developers.stellar.org/meetings/2024/09/12
