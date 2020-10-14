#!/usr/bin/env bash

$(dirname $0)/build.sh

( $(dirname $0)/target/debug/commonplace_gui_server ) &
( chromium --app="http://localhost:38841/index.html" --new-window ) &
wait -n
kill 0
