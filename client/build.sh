#!/bin/sh

set -ex

wasm-pack build --target web --out-dir ../dist/pkg
cp index.html ../dist
