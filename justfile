build package:
	cargo build -p {{package}} --all-features --all-targets

release package:
	cargo build --release -p {{package}}
	cp target/release/{{package}} {{package}}/

test package: (check package)
	cargo nextest run -p {{package}}

doc package:
	cargo doc --package {{package}}

test-all:
	cargo fmt
	cargo clippy --workspace --all-features --all-targets
	cargo nextest run
	cargo doc --workspace

check package:
	cargo fmt --package {{package}}
	cargo clippy --package {{package}} --all-features --all-targets

check-all:
	cargo fmt
	cargo clippy --workspace --all-features --all-targets
