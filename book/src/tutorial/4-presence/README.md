# Presence

| page | concept | needs |
|---|---|---|
| [4.1 Optional fields](./01-optional-fields.md) | `Option`: a field the packet may leave out | 1.1 |
| [4.2 Default values](./02-fallback-values.md) | `default`: the value a missing field takes | 4.1 |
| [4.3 Skipping defaults](./03-skipping-defaults.md) | `skip_if_default`, `skip_if`: write no item for a field at its default | 4.2 |
| [4.4 One-of fields](./04-one-of-several-keys.md) | `oneof`: one field, read from one of several keys | 1.4 |
