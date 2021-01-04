#!/bin/sh

set -ex

wasm-pack build --release --target web --out-dir ../dist/pkg
cp index.html ../dist
