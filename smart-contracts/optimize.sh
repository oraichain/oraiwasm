#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck > /dev/null && shellcheck "$0"

echo "Info: sccache stats before build"
sccache -s

contractdir="$1"

basedir=`pwd`
build_release="${3:-false}"
name=$(basename $contractdir)
cd $contractdir
echo "Building contract in $(realpath -m "$contractdir")"
(
    # Linker flag "-s" for stripping (https://github.com/rust-lang/cargo/issues/3483#issuecomment-431209957)
    # Note that shortcuts from .cargo/config are not available in source code packages from crates.io
    mkdir -p artifacts
    
    if [ $build_release == 'true' ]
    then
        RUSTFLAGS='-C link-arg=-s' RUSTC_WRAPPER=sccache cargo build --release --target-dir $basedir/target --target wasm32-unknown-unknown                 
        # wasm-optimize on all results        
        echo "Optimizing $wasm"
        wasm-opt -Os "$basedir/target/wasm32-unknown-unknown/release/$name.wasm" -o artifacts
    else 
        RUSTC_WRAPPER=sccache cargo build --target-dir $basedir/target --target wasm32-unknown-unknown
        cp "$basedir/target/wasm32-unknown-unknown/debug/$name.wasm" artifacts
    fi 
)
cd -



# # create hash
# (    
#     cd $package_folder
#     sha256sum -- *.wasm > checksums.txt
# )

build_schema="${2:-false}"
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