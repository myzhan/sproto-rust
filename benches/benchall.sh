#!/bin/bash
set -euo pipefail

COUNT=${1:-1000000}
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$(dirname "$SCRIPT_DIR")"
GO_DIR="${HOME}/src/gosproto/benchmark"

# ── Build ──
echo "Building Rust benchmark..."
(cd "$RUST_DIR" && cargo build --release --example benchmark 2>&1 | tail -1)

RUST_BIN="$RUST_DIR/target/release/examples/benchmark"

GO_BIN=""
if [ -d "$GO_DIR" ]; then
    echo "Building Go benchmark..."
    (cd "$GO_DIR" && go build -o benchmark . 2>&1)
    GO_BIN="$GO_DIR/benchmark"
else
    echo "Go benchmark not found at $GO_DIR, skipping cross-language comparison."
fi

echo ""

# ── Helper: extract cost (seconds) from output line ──
# Handles Go duration formats: "1.23s", "697.937ms", "67.125µs", "1m30.5s"
# and Rust format: "1.234567s"
extract_cost() {
    local raw
    raw=$(echo "$1" | awk -F'\t' '{print $4}')
    # Convert Go duration to seconds
    if echo "$raw" | grep -q 'µs$'; then
        echo "$raw" | sed 's/µs$//' | awk '{printf "%.9f", $1 / 1000000}'
    elif echo "$raw" | grep -q 'ms$'; then
        echo "$raw" | sed 's/ms$//' | awk '{printf "%.9f", $1 / 1000}'
    elif echo "$raw" | grep -q 'm[0-9]'; then
        # Format like "1m30.5s" → minutes + seconds
        local mins secs
        mins=$(echo "$raw" | sed 's/m.*//')
        secs=$(echo "$raw" | sed 's/.*m//' | sed 's/s$//')
        awk "BEGIN { printf \"%.9f\", $mins * 60 + $secs }"
    else
        echo "$raw" | sed 's/s$//'
    fi
}

# ── Helper: compute ratio ──
ratio() {
    local a="$1" b="$2"
    if [ -z "$b" ] || [ "$b" = "0" ] || [ "$b" = "N/A" ]; then
        echo "N/A"
    else
        awk "BEGIN { printf \"%.1fx\", $a / $b }"
    fi
}

# ── Helper: format seconds ──
fmt_time() {
    local t="$1"
    if [ "$t" = "N/A" ]; then
        echo "N/A"
    else
        awk "BEGIN { printf \"%.4fs\", $t }"
    fi
}

# ── Run Rust benchmarks ──
echo "Running Rust benchmarks (count: $COUNT)..."

run_rust() {
    local mode="$1" api="$2"
    local output
    output=$("$RUST_BIN" --count="$COUNT" --mode="$mode" --api="$api" 2>/dev/null)
    extract_cost "$output"
}

rust_encode=$(run_rust encode direct)
rust_decode=$(run_rust decode direct)
rust_encode_pack=$(run_rust encode_pack direct)
rust_unpack_decode=$(run_rust unpack_decode direct)

rust_derive_encode=$(run_rust encode derive)
rust_derive_decode=$(run_rust decode derive)
rust_derive_encode_pack=$(run_rust encode_pack derive)
rust_derive_unpack_decode=$(run_rust unpack_decode derive)

# ── Run Go benchmarks ──
go_encode_reflect="N/A"
go_encode_codec="N/A"
go_decode_reflect="N/A"
go_decode_codec="N/A"

if [ -n "$GO_BIN" ]; then
    echo "Running Go benchmarks (count: $COUNT)..."

    run_go() {
        local mode="$1" method="$2"
        local output
        output=$("$GO_BIN" -count="$COUNT" -mode="$mode" -method="$method" 2>/dev/null)
        extract_cost "$output"
    }

    go_encode_reflect=$(run_go encode reflect)
    go_encode_codec=$(run_go encode codec)
    go_decode_reflect=$(run_go decode reflect)
    go_decode_codec=$(run_go decode codec)
fi

echo ""

# ── Print Report ──
SEP="================================================================"
echo "$SEP"
echo "sproto benchmark (count: $COUNT)"
echo "$SEP"
echo ""

echo "Rust API comparison (AddressBook — nested structs):"
printf "%-14s | %-12s | %-12s | %s\n" \
    "Scenario" "Direct" "Derive" "Derive/Direct"
printf "%-14s-+-%-12s-+-%-12s-+-%s\n" \
    "--------------" "------------" "------------" "-------------"

print_row_cmp() {
    local label="$1" t_direct="$2" t_derive="$3"
    printf "%-14s | %-12s | %-12s | %s\n" \
        "$label" \
        "$(fmt_time "$t_direct")" \
        "$(fmt_time "$t_derive")" \
        "$(ratio "$t_derive" "$t_direct")"
}

print_row_cmp "encode"        "$rust_encode"        "$rust_derive_encode"
print_row_cmp "decode"        "$rust_decode"        "$rust_derive_decode"
print_row_cmp "encode+pack"   "$rust_encode_pack"   "$rust_derive_encode_pack"
print_row_cmp "unpack+decode" "$rust_unpack_decode" "$rust_derive_unpack_decode"

if [ -n "$GO_BIN" ]; then
    echo ""
    echo "Cross-language comparison (Go encode=encode+pack, Go decode=unpack+decode):"
    printf "%-14s | %-12s | %-12s | %-12s | %-12s\n" \
        "Scenario" "Go(reflect)" "Go(codec)" "Rust(direct)" "Rust(derive)"
    printf "%-14s-+-%-12s-+-%-12s-+-%-12s-+-%s\n" \
        "--------------" "------------" "------------" "------------" "------------"
    printf "%-14s | %-12s | %-12s | %-12s | %-12s\n" \
        "encode+pack" \
        "$(fmt_time "$go_encode_reflect")" \
        "$(fmt_time "$go_encode_codec")" \
        "$(fmt_time "$rust_encode_pack")" \
        "$(fmt_time "$rust_derive_encode_pack")"
    printf "%-14s | %-12s | %-12s | %-12s | %-12s\n" \
        "unpack+decode" \
        "$(fmt_time "$go_decode_reflect")" \
        "$(fmt_time "$go_decode_codec")" \
        "$(fmt_time "$rust_unpack_decode")" \
        "$(fmt_time "$rust_derive_unpack_decode")"

    echo ""
    echo "Speedup ratios (X / Rust-direct, higher = Rust-direct faster):"
    printf "%-14s | %-16s | %-16s | %s\n" \
        "Scenario" "go_reflect/rust" "go_codec/rust" "derive/direct"
    printf "%-14s-+-%-16s-+-%-16s-+-%s\n" \
        "--------------" "----------------" "----------------" "-------------"

    printf "%-14s | %-16s | %-16s | %s\n" \
        "encode+pack" \
        "$(ratio "$go_encode_reflect" "$rust_encode_pack")" \
        "$(ratio "$go_encode_codec" "$rust_encode_pack")" \
        "$(ratio "$rust_derive_encode_pack" "$rust_encode_pack")"
    printf "%-14s | %-16s | %-16s | %s\n" \
        "unpack+decode" \
        "$(ratio "$go_decode_reflect" "$rust_unpack_decode")" \
        "$(ratio "$go_decode_codec" "$rust_unpack_decode")" \
        "$(ratio "$rust_derive_unpack_decode" "$rust_unpack_decode")"
fi

echo ""
echo "$SEP"
