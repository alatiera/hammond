#!/bin/sh

cargo build --release && cp $1/target/release/hammond-gtk $2