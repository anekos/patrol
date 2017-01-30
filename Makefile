
test:
	cargo build --release

publish:
	cargo package
	cargo publish
