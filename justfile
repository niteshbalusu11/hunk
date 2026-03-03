start:
    cargo run

build:
    cargo build

release:
    cargo build --release

dev:
    bacon

bundle:
    cargo bundle --release

prod:
    osascript -e 'tell application "Hunk" to quit' || true
    just bundle
    open target/release/bundle/osx/Hunk.app
