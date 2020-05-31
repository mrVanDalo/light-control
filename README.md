# rust-iot

light-control for home

## How to debug and run

```shell script
export RUST_LOG=rust_iot=trace
cargo run \
  --color=always \
  --package rust-iot \
  --bin rust-iot \
  -- \
  examples/my-home.json \
  |& tee `date +%Y-%m-%d_%H%M%S`.log
```

