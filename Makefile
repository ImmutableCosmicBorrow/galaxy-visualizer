.PHONY: fmt lint test ci doc

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings -W clippy::pedantic -A unused

test:
	cargo test

coverage:
	cargo tarpaulin --out Lcov

all: fmt lint test

doc:
	cargo doc

run: 
	cargo build && cargo run	

release:
	cargo build --release && cargo run