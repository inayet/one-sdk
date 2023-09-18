#!/bin/sh

base=$(dirname "$0")
base=$(realpath "$base")
cd "$base"

MAKE_FLAGS=${1}

cd "$base/../.."
make build_nodejs_host CORE_PROFILE=release $MAKE_FLAGS
cd "$base"
node --no-warnings --experimental-wasi-unstable-preview1 ./client.mjs >data/node_data.csv

cd "$base/../.."
make build_python_host CORE_PROFILE=release $MAKE_FLAGS
cd "$base"
source ../../packages/python_host/venv/bin/activate
python3 -m pip install psutil
python3 client.py >data/python_data.csv
