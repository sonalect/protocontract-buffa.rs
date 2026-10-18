# Example: references to a service

`proto/refs/v1/refs.proto` declares a service, `Clock`, and a message that
stands for an implementation of it:

```protobuf
message ClockRef {
  option (protocontract.v1.service) = "refs.v1.Clock";
}
```

`Alarm` holds references in a field and a list, and `Alarms` takes one
(`UseClock`) and gives one back (`CurrentClock`). `buf.gen.yaml` maps
`ClockRef` to the type the plugin emits, `crate::traits::clock_ref::ClockRef`,
in both plugins.

`api/tests/refs.rs` calls it all in both forms: a blocking and an async
clock in one message, a reference through the `Inline` bridge, the form
conversions, what encoding and JSON do to a reference, and when it fails
with `UNBOUND` or `NO_RUNTIME`.

```bash
bazel run //examples/refs:generate   # regenerate api/src/generated
cargo test -p refs-api
```
