#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck > /dev/null && shellcheck "$0"

export PATH=$PATH:/root/.cargo/bin

echo "Info: sccache stats before build"
sccache -s

contractdir="$1"

# There are two cases here
# 1. All contracts (or one) are included in the root workspace  (eg. `cosmwasm-template`, `cosmwasm-examples`, `cosmwasm-plus`)
#    In this case, we pass no argument, just mount the proper directory.
# 2. Contracts are excluded from the root workspace, but import relative paths from other packages (only `cosmwasm`).
#    In this case, we mount root workspace and pass in a path `docker run <repo> ./contracts/hackatom`

# This parameter allows us to mount a folder into docker container's "/code"
# and build "/code/contracts/mycontract".
# Note: if contractdir is "." (default in Docker), this ends up as a noop

name=$(basename $contractdir)

echo "Building contract in $(realpath -m "$contractdir")"
(
    # Linker flag "-s" for stripping (https://github.com/rust-lang/cargo/issues/3483#issuecomment-431209957)
    # Note that shortcuts from .cargo/config are not available in source code packages from crates.io
    RUSTFLAGS='-C link-arg=-s' RUSTC_WRAPPER=sccache cargo build -q --release --target wasm32-unknown-unknown -p $name    
)

wasm=$name.wasm 
package_folder=$contractdir/artifacts/
# wasm-optimize on all results
mkdir -p $package_folder
echo "Optimizing $wasm"
wasm-opt -Os "target/wasm32-unknown-unknown/release/$wasm" -o "$package_folder/$wasm"


# create hash
(    
    cd $package_folder
    sha256sum -- *.wasm > checksums.txt
)

build_schema="${2:-true}"
# create schema if there is
if [ $build_schema == 'true' ]
then
    echo "Creating schema in $(realpath -m "$contractdir")"
    (    
        RUSTC_WRAPPER=sccache cargo run -q --release -p $name --example schema
        mv schema $package_folder
    )
fi

echo "Info: sccache stats after build"
sccache -s

echo "done"