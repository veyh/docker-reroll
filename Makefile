name := $(shell dasel -f Cargo.toml package.name)

.PHONY: dev debug release lint test clean deploy

dev:
	while true; do fd . | entr -ccd make lint test debug; done

debug:
	mkdir -p dist
	cargo build --target x86_64-unknown-linux-musl
	ln -f "target/x86_64-unknown-linux-musl/debug/${name}" "dist/"

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
	rclone copyto \
		"target/x86_64-unknown-linux-musl/release/${name}" \
		"r2:/cdn-soupbawx-com/${name}"
