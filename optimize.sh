#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck > /dev/null && shellcheck "$0"

echo "Info: sccache stats before build"
sccache -s

contractdir=$(realpath -m "$1")

basedir=`pwd`
build_release="${3:-true}"
name=$(basename $contractdir)
cd $contractdir
echo "Building contract in $contractdir"
(
    # Linker flag "-s" for stripping (https://github.com/rust-lang/cargo/issues/3483#issuecomment-431209957)
    # Note that shortcuts from .cargo/config are not available in source code packages from crates.io
    mkdir -p artifacts
    
    if [ $build_release == 'true' ]
    then
        RUSTFLAGS='-C link-arg=-s' RUSTC_WRAPPER=sccache cargo build -q --release --target-dir $basedir/target --target wasm32-unknown-unknown                 
        # wasm-optimize on all results        
        echo "Optimizing $name.wasm"
        wasm-opt -Os "$basedir/target/wasm32-unknown-unknown/release/$name.wasm" -o artifacts/$name.wasm
    else 
        RUSTC_WRAPPER=sccache cargo build -q --target-dir $basedir/target --target wasm32-unknown-unknown
        echo "RUSTC_WRAPPER=sccache cargo build -q --target-dir $basedir/target --target wasm32-unknown-unknown"
        cp "$basedir/target/wasm32-unknown-unknown/debug/$name.wasm" artifacts
    fi 
)

build_schema="${2:-false}"
# create schema if there is
if [ $build_schema == 'true' ]
then
    echo "Creating schema in $contractdir"
    (            
        RUSTC_WRAPPER=sccache cargo run -q --example schema  --target-dir $basedir/target  
        # put in artifacts for simulator
        mv schema artifacts
    )
fi

echo "Info: sccache stats after build"
sccache -s

echo "done"