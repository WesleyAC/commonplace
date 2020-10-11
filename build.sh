#!/usr/bin/env bash

BUILD_MODE=debug
BUILD_MODE_FLAG=

for arg in "$@"
do
    case $arg in
        --release)
	BUILD_MODE=release
        BUILD_MODE_FLAG=--release
        shift
        ;;
        *)
        shift
        ;;
    esac
done

BASE_DIR=$(cd `dirname $0` && pwd)
cd $BASE_DIR

set -ex

cd $BASE_DIR/commonplace_gui_client
cargo +nightly build $BUILD_MODE_FLAG --target wasm32-unknown-unknown
wasm-bindgen --target web $BASE_DIR/target/wasm32-unknown-unknown/$BUILD_MODE/commonplace_gui_client.wasm --out-dir $BASE_DIR/static/

cd $BASE_DIR/commonplace_gui
cargo +nightly build $BUILD_MODE_FLAG

cd $BASE_DIR
