# SPDX-License-Identifier: AGPL-3.0-or-later

# One entry point for the project commands. `just check` runs the same steps as
# the Linux job in .github/workflows/ci.yml, in the same order. Run `just` to
# list the recipes.

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# List the recipes
default:
    @just --list --unsorted

# Install the JavaScript dependencies from the lock file
setup:
    bun install --frozen-lockfile

# Build the server program that the desktop app needs
[working-directory: 'src/apps/desktop']
sidecar:
    bun scripts/prepare-sidecar.ts

# Run the desktop app in development mode
[working-directory: 'src/apps/desktop']
dev: sidecar
    bun run tauri dev

# Build the desktop app without installers
[working-directory: 'src/apps/desktop']
build:
    bun run tauri build --no-bundle

# Run the Rust tests and the webview unit tests
test: test-rust test-web

# Run the Rust tests of the whole workspace
test-rust: sidecar
    cargo test --workspace

# Run the webview unit tests
[working-directory: 'src/apps/desktop']
test-web:
    bun test src

# Install the Chromium browser and its system packages for Playwright
[working-directory: 'src/apps/desktop']
e2e-install:
    bunx playwright install --with-deps chromium

# Run the Playwright tests (run e2e-install once first)
[working-directory: 'src/apps/desktop']
e2e:
    bun run e2e

# Run every lint and format check
lint: lint-rust boundaries lint-web

# Check Rust format and run clippy
lint-rust: sidecar
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

# Check the module boundary rules
boundaries:
    scripts/check-boundaries.sh --self-test
    scripts/check-boundaries.sh

# Type-check, lint, and format-check the webview
[working-directory: 'src/apps/desktop']
lint-web:
    bun run check
    bun run lint
    bun run format:check

# Format the Rust and webview code
fmt:
    cargo fmt --all
    cd src/apps/desktop && bunx prettier --write .

# Run everything the Linux CI job runs, in the order of ci.yml
check: sidecar lint-rust test-rust boundaries setup lint-web test-web e2e-install e2e build

# Remove build output
clean:
    cargo clean
    rm -rf src/apps/desktop/dist src/apps/desktop/test-results src/apps/desktop/src-tauri/binaries
