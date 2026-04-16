#!/bin/bash
# Build sproto.so from the reference C implementation and generate test fixtures.
#
# Prerequisites:
#   - Lua 5.4 installed (e.g. via `brew install lua`)
#   - gcc available
#   - ~/github/sproto contains the reference C/Lua sproto source

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPROTO_DIR="$HOME/github/sproto"
LUA_INCLUDE="$(pkg-config --cflags lua 2>/dev/null || echo "-I/opt/homebrew/include/lua")"

echo "=== Building sproto.so ==="
cd "$SPROTO_DIR"
gcc -O2 -Wall -bundle -undefined dynamic_lookup $LUA_INCLUDE -o sproto.so sproto.c lsproto.c
echo "Built: $SPROTO_DIR/sproto.so"

echo ""
echo "=== Generating test fixtures ==="
cd "$SCRIPT_DIR"
LUA_CPATH="$SPROTO_DIR/?.so;/opt/homebrew/lib/lua/5.4/?.so" LUA_PATH="$SPROTO_DIR/?.lua" lua generate.lua

echo ""
echo "=== Done ==="
ls -la "$SCRIPT_DIR"/*.bin
