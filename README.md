# rust-iot

light-control for home

## How to debug and run

This generates a
* `2020-05-23_233423.log`: log file of that run
* `2020-05-23_233423.sh`: replay script, to replay and verify false behavior
* `2020-05-23_233423.json`: configuration used for that run

```shell script
export RUST_LOG=rust_iot=trace
TIME_STAMP=`date +%Y-%m-%d_%H%M%S`
cargo run \
  --color=always \
  --package rust-iot \
  --bin rust-iot \
  -- \
  examples/my-home.json \
  --replay-config ./${TIME_STAMP}.json \
  --replay-script ./${TIME_STAMP}.sh \
  |& tee ./${TIME_STAMP}.log
```

