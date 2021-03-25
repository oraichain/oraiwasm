# testcase-contract-template

template for the oracle test case smart contract

## update rustup version >= 1.48.0

```bash
rustup update stable
rustup default stable-x86_64-unknown-linux-gnu
```

## needs cargo-generate --features vendored-openssl

```bash
cargo install --git https://github.com/cargo-generate/cargo-generate.git --features vendored-openssl
```

## generate the template

```bash
cargo generate --git https://github.com/oraichain/testcase-contract-template.git --name PROJECT_NAME --force
```