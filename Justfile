#!/usr/bin/env just --justfile

alias t := test

test:
    cargo +stable test
    cargo +nightly test --features type-safe

clippy:
    cargo +stable clippy
    cargo +stable clippy --release
    cargo +nightly clippy --features type-safe
    cargo +nightly clippy --features type-safe --release
