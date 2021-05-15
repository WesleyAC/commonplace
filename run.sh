#!/usr/bin/env bash

set -e

$(dirname $0)/build.sh $@

( RUST_BACKTRACE=1 $(dirname $0)/target/debug/commonplace-gui ) &
( chromium --app="http://localhost:38841/index.html" --new-window ) &
wait -n
kill 0
