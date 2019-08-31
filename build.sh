#!/bin/bash

cd examples/
erlc temp.erl
cd ..

cargo clippy
cargo t
