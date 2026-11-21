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

## Validation

`Alarm` and `Tick` carry protovalidate's rules
(`buf/validate/validate.proto`):

| Field | Rule | Fails with |
| - | - | - |
| `Alarm.label` | `required`, `string.min_len = 1` | a missing or an empty label |
| `Alarm.clock` | `required` | no clock |
| `Alarm.backups` | `repeated.max_items = 3` | four backups or more |
| `Tick.unix_ms` | `int64.gte = 0` | a time before the epoch |

`buf.gen.yaml` runs `protoc-gen-protovalidate-buffa` next to this plugin,
and gives this plugin `validate=true`: the validator of `Alarm` calls
`validate()` on `clock` and on every backup, so `ClockRef` needs
`Validate`, and the plugin emits it, always valid. `Alarms.Set` fails with
`ALARMS_ERROR_CODE_INVALID_ALARM` when the alarm breaks a rule, with
protovalidate's violations as the error's source.

A rule sees the field, not the implementation. An unbound `ClockRef` is
set, so it meets `required`; it fails only when a form is taken, with
`UNBOUND`. The protos of protovalidate come from mise (`http:protovalidate`):
`mise run protos` stages them under `.cache/proto`, which `buf.yaml` lists
as a module. Nothing comes from the Buf Schema Registry.

## Tests

`api/tests/refs.rs` calls it all in both forms: a blocking and an async
clock in one message, a reference through the `Inline` bridge, the form
conversions, what encoding and JSON do to a reference, when it fails
with `UNBOUND` or `NO_RUNTIME`, and each rule of `Alarm` and `Tick`.

```bash
mise run generate   # regenerate api/src/generated (and the rest), from the repository root
cargo test -p refs-api
```
