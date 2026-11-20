# Graphite mode spike program

The measurement program behind [the spike note](../../docs/graphite-mode-spike.md). It is kept as text files, because it is a throwaway crate that must stay outside the workspace and its lints and dependency rules.

To run it, make a new directory outside this repository and copy the files in:

```sh
mkdir graphite-spike && cd graphite-spike && mkdir src
cp /path/to/lunatui/scripts/graphite-spike/Cargo.toml.txt Cargo.toml
cp /path/to/lunatui/scripts/graphite-spike/Cargo.lock.txt Cargo.lock
cp /path/to/lunatui/scripts/graphite-spike/main.rs.txt src/main.rs
cargo run --release --locked
```

Pass a TTF path as the first argument to use a different font. Run it on a quiet machine: other work running at the same time changes the numbers.
