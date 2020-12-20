#!/bin/sh

set -ex

wasm-pack build --target web
py -m http.server
