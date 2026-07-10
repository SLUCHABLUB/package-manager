test:
    cargo test
    @just ui-tests/test-all

clean:
    cargo clean
    @just ui-tests/clean-all
