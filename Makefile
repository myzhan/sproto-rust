.PHONY: all build test benchmark clean doc check fmt lint lua-build lua-test

all: build

# Build the library
build:
	cargo build

# Build in release mode
release:
	cargo build --release

# Run all tests
test:
	cargo test --all-features

# Run benchmarks
benchmark:
	cargo bench --bench sproto_bench

# Run a quick benchmark (fewer iterations)
benchmark-quick:
	cargo bench --bench sproto_bench -- --quick

# Clean build artifacts
clean:
	cargo clean

# Generate documentation
doc:
	cargo doc --all-features --no-deps

# Open documentation in browser
doc-open:
	cargo doc --all-features --no-deps --open

# Check code without building
check:
	cargo check --all-features

# Format code
fmt:
	cargo fmt

# Check formatting
fmt-check:
	cargo fmt -- --check

# Run clippy lints
lint:
	cargo clippy --all-features -- -D warnings

# Run all CI checks (format, lint, test)
ci: fmt-check lint test

# Install development tools
install-tools:
	rustup component add rustfmt clippy

# Build Lua binding
lua-build:
	cd sproto-lua && cargo build --release
	cp target/release/libsproto_lua.dylib sproto-lua/sproto_lua.so || \
	cp target/release/libsproto_lua.so sproto-lua/sproto_lua.so || true

# Test Lua binding
lua-test: lua-build
	cd sproto-lua && LUA_CPATH="./?.so;;" busted tests/spec.lua
