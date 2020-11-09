#!/usr/bin/env bash

BUILD_MODE=build
BUILD_MODE_FLAG=

for arg in "$@"
do
    case $arg in
        --release)
	BUILD_MODE=build_release
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
cargo make $BUILD_MODE

cd $BASE_DIR/commonplace_gui_server
cargo +nightly build $BUILD_MODE_FLAG

cd $BASE_DIR
