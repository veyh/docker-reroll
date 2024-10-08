name := $(shell dasel -f Cargo.toml package.name)
version := $(shell dasel -f Cargo.toml package.version)

.PHONY: dev debug release lint test clean deploy

dev:
	while true; do fd . | entr -ccd make lint test debug; done

debug:
	mkdir -p dist
	cargo build
	ln -f "target/debug/${name}" "dist/"

release:
	mkdir -p dist
	cargo build --release --target x86_64-unknown-linux-musl
	ln -f "target/x86_64-unknown-linux-musl/release/${name}" "dist/"

lint:
	cargo clippy

test:
	cargo test

clean:
	cargo clean

deploy: release
	scripts/deploy.sh "${name}" "${version}"
