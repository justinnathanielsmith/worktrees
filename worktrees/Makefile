.PHONY: check fmt clippy test coverage clean build

check:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

coverage:
	@if command -v cargo-tarpaulin >/dev/null; then 
		cargo tarpaulin --ignore-tests; 
	else 
		echo "cargo-tarpaulin not installed. Run: cargo install cargo-tarpaulin"; 
	fi

build:
	cargo build --release

clean:
	cargo clean
