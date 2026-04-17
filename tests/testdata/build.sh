#!/bin/bash
# Build sproto.so from the reference C implementation and generate test fixtures.
#
# Prerequisites:
#   - Lua installed (defaults to ~/local/lua-5.5.0/src)
#   - gcc available
#   - ~/github/sproto contains the reference C/Lua sproto source

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPROTO_DIR="$HOME/github/sproto"
LUA_DIR="${LUA_DIR:-$HOME/local/lua-5.5.0/src}"
LUA_INCLUDE="-I$LUA_DIR"
LUA="$LUA_DIR/lua"

echo "=== Building sproto.so ==="
cd "$SPROTO_DIR"
gcc -O2 -Wall -bundle -undefined dynamic_lookup $LUA_INCLUDE -o sproto.so sproto.c lsproto.c
echo "Built: $SPROTO_DIR/sproto.so"

echo ""
echo "=== Generating test fixtures ==="
cd "$SCRIPT_DIR"
LUA_CPATH="$SPROTO_DIR/?.so" LUA_PATH="$SPROTO_DIR/?.lua" "$LUA" generate.lua

echo ""
echo "=== Done ==="
ls -la "$SCRIPT_DIR"/*.bin
