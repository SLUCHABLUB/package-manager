test:
    cargo test
    @just ui-tests/test-all

clean:
    cargo clean
    @just ui-tests/clean-all

offline-clean-tests:
    @just ui-tests/offline-clean-all
