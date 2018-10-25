# arm-rs
[![Build Status](https://travis-ci.org/cpearce/arm-rs.svg?branch=master)](https://travis-ci.org/cpearce/arm-rs)

An implementation of association rule mining via the FPGrowth algorithm in Rust.

This finds relationships of the form "people who buy X also buy Y",
and also determines the strengths (confidence, lift, support) of those
relationships.

This implementation parallelizes FPGrowth using the [Rayon Rust crate](https://crates.io/crates/rayon).

For an overview of assocation rule mining,
see Chapter 5 of Introduction to Data Mining, Kumar et al:
[Association Analysis: Basic Concepts and Algorithms](https://www-users.cs.umn.edu/~kumar001/dmbook/ch5_association_analysis.pdf).

To build, install Rust from [rustup.rs](https://rustup.rs/), then to build run:

    cargo build --release

To run:

    cargo run --release -- $ARGS

For example:

    cargo run --release -- \
        --input datasets/kosarak.csv \
        --output rules.txt \
        --min-support 0.05 \
        --min-confidence 0.05 \
        --min-lift 5

Input files are in CSV format, that is, one transaction of items per line, items separated by commas.

To run tests:

    cargo test

Auto-format code via:

    cargo fmt
