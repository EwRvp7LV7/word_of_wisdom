#!/bin/bash
set -e
cargo build --release
killall server &>/dev/null || true
echo "Running TCP server"
./target/release/server &
server_pid=$!
set +e
sleep 0.5
echo "Running TCP client"
trap "kill $server_pid" SIGINT
./target/release/client
kill $server_pid
