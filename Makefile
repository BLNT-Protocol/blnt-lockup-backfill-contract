SOURCE_REPO_META := source_repo=github:BLNT-Protocol/blnt-lockup-backfill-contract
WASM := target/wasm32v1-none/optimized/blnt_lockup_backfill_contract.wasm

.PHONY: build verify-source-metadata test check fmt clean

build:
	stellar contract build --package blnt-lockup-backfill-contract --locked \
		--meta $(SOURCE_REPO_META) \
		--out-dir target/wasm32v1-none/optimized
	$(MAKE) --no-print-directory verify-source-metadata

verify-source-metadata:
	@stellar contract info meta --wasm $(WASM) 2>/dev/null | \
		grep -Fq "source_repo: github:BLNT-Protocol/blnt-lockup-backfill-contract" || \
		{ printf 'error: missing source_repo metadata in lockup backfill WASM\n' >&2; exit 1; }

test:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features --locked -- -D warnings
	cargo test --all-features --locked

check:
	cargo check --all-targets --all-features --locked

fmt:
	cargo fmt --all

clean:
	cargo clean
