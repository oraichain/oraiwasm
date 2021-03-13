# Create a new Oraichain smart contract tutorial

This tutorial demonstrates a simple way to generate and build a smart contract to run on Oraichain

## Generate a smart contract

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

## Build a smart contract

### 1. Switch your Rust version to the cosmwasm version to build the smart contracts

Type:

```bash
rustup default 1.47.0-x86_64-unknown-linux-gnu
```

### 2. Build the smart contract

Enter a smart contract directory and type:

```bash
optimize.sh <directory-name>
```

Example:

```bash
optimize.sh cv009
```