set ignore-comments

clean-tests:
    rm -rf target/tmp/build_bat

initialise-rustup:
    HOME='{{ justfile_directory() }}/target/tmp/build_bat/home' rustup default stable

cross-check:
    # TODO: Test the ones with no docker images.

    # x86
    @just cross check x86_64-unknown-linux-musl
    @just cross check x86_64-unknown-linux-gnu
    @just cross check x86_64-unknown-freebsd
    #@just cross check x86_64-unknown-openbsd
    @just cross check x86_64-unknown-netbsd
    #@just cross check x86_64-unknown-redox

    # ARM
    @just cross check aarch64-unknown-linux-musl
    @just cross check aarch64-unknown-linux-gnu
    #@just cross check aarch64-unknown-freebsd
    #@just cross check aarch64-unknown-openbsd
    #@just cross check aarch64-unknown-netbsd
    #@just cross check aarch64-unknown-redox

    # Proprietary garbage.
    #@just cross check aarch64-apple-darwin

    # Proprietary esoteric garbage.
    #@just cross check x86_64-pc-windows-msvc
    @just cross check x86_64-pc-windows-gnu

[private]
cross command target:
    cross '{{command}}' --target '{{target}}' --target-dir '{{justfile_directory()}}/target/cross/{{target}}'
