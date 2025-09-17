set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

cargo-toml := read("Cargo.toml")
version := replace_regex(cargo-toml, '(?ms).*^version = "([^"]+)".*', "$1")
release-dir := "dist" / version
linux-binary := "jsonwatch-v" + version + "-linux-x86_64"
win32-binary := "jsonwatch-v" + version + "-win32.exe"
checksum-file := "SHA256SUMS.txt"
ssh-key := x"~/.ssh/git"
tclsh := "tclsh"
export JSONWATCH_COMMAND := "target/debug/jsonwatch"

default: test

version:
    @echo {{ version }}

build:
    cargo build

[unix]
release: release-linux release-windows
    #! /bin/sh
    cd {{ release-dir }}
    sha256sum {{ linux-binary }} {{ win32-binary }} > {{ checksum-file }}
    ssh-keygen -Y sign -n file -f {{ ssh-key }} {{ checksum-file }}

[unix]
release-linux:
    mkdir -p {{ release-dir }}
    cargo build --release --target x86_64-unknown-linux-musl
    cp target/x86_64-unknown-linux-musl/release/jsonwatch {{ release-dir / linux-binary }}
    strip {{ release-dir / linux-binary }}

[unix]
release-windows:
    mkdir -p {{ release-dir }}
    cargo build --release --target i686-pc-windows-gnu
    cp target/i686-pc-windows-gnu/release/jsonwatch.exe {{ release-dir / win32-binary }}
    strip {{ release-dir / win32-binary }}

[unix]
test: build test-unit test-e2e

[windows]
test: build test-unit

# The end-to-end tests use Expect and do not work on Windows.
[unix]
test-e2e:
    {{ tclsh }} tests/e2e.test

test-unit:
    cargo test
