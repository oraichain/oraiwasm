# Create a new Oraichain smart contract tutorial

This tutorial demonstrates a simple way to generate and build a smart contract to run on Oraichain

## Build a smart contract

### 1. Switch your Rust version to the cosmwasm version to build the smart contracts

Type:

```bash
rustup default 1.47.0-x86_64-unknown-linux-gnu && cp optimize.sh /usr/local/bin/optimize.sh && chmod +x /usr/local/bin/optimize.sh
```

### 2. Build the smart contract

In the /code directory, type:

```bash
optimize.sh <parent-dir> <dir>
```

Example:

```bash
optimize.sh nlp nl002
```

After building, you can start deploying your smart contracts (using either CLI - oraicli from Cosmosjs repo or UI wallet)

## Generate a smart contract (optional)

There are three types of smart contracts: [data source](https://github.com/oraichain/datasource-contract-template.git), [test case](https://github.com/oraichain/testcase-contract-template.git) and [oracle script](https://github.com/oraichain/oscript-contract-template.git) corresponding to three templates. Please follow the below steps to generate a smart contract. To generate a smart contract, you need a Rust stable version.

### 1. Update rustup version >= 1.48.0

```bash
rustup update stable
rustup default stable-x86_64-unknown-linux-gnu
```

### 2. Install cargo-generate --features vendored-openssl

```bash
cargo install --git https://github.com/cargo-generate/cargo-generate.git --features vendored-openssl
```

### 3. Generate the template

```bash
cargo generate --git https://github.com/oraichain/<TEMPLATE_NAME>-contract-template.git --name <PROJECT_NAME> --force
```