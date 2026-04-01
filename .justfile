# Build release binary
build:
    @cargo build --release

# Quick type/syntax check
check:
    @cargo check

# Run all unit tests
test:
    @cargo test

# Format code
fmt:
    @cargo fmt

# Run clippy lints
lint:
    @cargo clippy

# Run all CI checks: formatting, lints, tests
ci:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "checking formatting..."
    cargo fmt --check
    echo "running clippy..."
    cargo clippy -- -D warnings
    echo "running tests..."
    cargo test
    echo "all checks passed."

# Run with test.json for quick manual testing
run:
    @cargo run < test.json

# Clean build artifacts
clean:
    @cargo clean

# Build, sign, install to ~/.claude/, and configure settings.json
install:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release
    mkdir -p ~/.claude
    cp target/release/statusline ~/.claude/cc-statusline-rs
    chmod +x ~/.claude/cc-statusline-rs
    xattr -cr ~/.claude/cc-statusline-rs
    codesign -fs - ~/.claude/cc-statusline-rs
    settings=~/.claude/settings.json
    if [[ -f "$settings" ]]; then
        tmp=$(mktemp)
        jq '.statusLine = {"type": "command", "command": "~/.claude/cc-statusline-rs"}' "$settings" > "$tmp" \
            && mv "$tmp" "$settings"
        echo "updated $settings"
    else
        echo '{"statusLine": {"type": "command", "command": "~/.claude/cc-statusline-rs"}}' > "$settings"
        echo "created $settings"
    fi
    echo "installed to ~/.claude/cc-statusline-rs"
