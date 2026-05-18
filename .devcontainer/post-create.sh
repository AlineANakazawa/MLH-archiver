#!/usr/bin/env bash

rustup component add clippy rustfmt
cargo fetch

prek install
