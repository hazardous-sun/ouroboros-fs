#!/usr/bin/env bash
set -Eeuo pipefail

# How many nodes to run
NODES_AMOUNT=${NODES_AMOUNT:-3}
# First port to use
BASE_PORT=${BASE_PORT:-7000}

if (( NODES_AMOUNT < 1 )); then
  echo "NODES_AMOUNT must be >= 1" >&2
  exit 1
fi

# Build the project
cargo build --quiet

pids=()
ports=()

# Start the nodes
for ((i=0; i<NODES_AMOUNT; i++)); do
  port=$((BASE_PORT + i))
  ./target/debug/rust_socket_server "$port" & pids+=($!)
  ports+=("$port")
done

cleanup() {
  if ((${#pids[@]})); then
    kill "${pids[@]}" 2>/dev/null || true
    wait "${pids[@]}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# Give the servers a brief moment to start
sleep 1

# Set the "next_node" for each node, making it circular
for ((i=0; i<NODES_AMOUNT; i++)); do
  src_port=${ports[i]}
  next_port=${ports[(i+1)%NODES_AMOUNT]}
  printf 'SET_NEXT 127.0.0.1:%d\n' "$next_port" | nc -N 127.0.0.1 "$src_port"
done

# Block until user types 'quit'
while IFS= read -r -p "Type 'quit' to exit: " input; do
  [[ "$input" == "quit" ]] && break
done
