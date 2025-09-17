set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

CHECKSUM_FILE := "SHA256SUMS.txt"
LINUX_BINARY := "jsonwatch-linux-x86_64"
WIN32_BINARY := "jsonwatch-win32.exe"
SSH_KEY := x"~/.ssh/git"
TCLSH := "tclsh"
export JSONWATCH_COMMAND := "target/debug/jsonwatch"

default: test

build:
    cargo build

[unix]
release: release-linux release-windows
    sha256sum {{ LINUX_BINARY }} {{ WIN32_BINARY }} > {{ CHECKSUM_FILE }}
    ssh-keygen -Y sign -n file -f {{ SSH_KEY }} {{ CHECKSUM_FILE }} > {{ CHECKSUM_FILE }}.sig

[unix]
release-linux:
    cargo build --release --target x86_64-unknown-linux-musl
    cp target/x86_64-unknown-linux-musl/release/jsonwatch {{ LINUX_BINARY }}
    strip {{ LINUX_BINARY }}

[unix]
release-windows:
    cargo build --release --target i686-pc-windows-gnu
    cp target/i686-pc-windows-gnu/release/jsonwatch.exe {{ WIN32_BINARY }}
    strip {{ WIN32_BINARY }}

[unix]
test: build test-unit test-e2e

[windows]
test: build test-unit

# The end-to-end tests use Expect and do not work on Windows.
[unix]
test-e2e:
    {{ TCLSH }} tests/e2e.test

test-unit:
    cargo test
