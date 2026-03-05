# alloy-rlp-derive

This crate provides derive macros for traits defined in
[`alloy-rlp`](https://docs.rs/alloy-rlp). See that crate's documentation for
more information.

This library also supports up to 1 `#[rlp(default)]` in a struct, which is
similar to [`#[serde(default)]`](https://serde.rs/field-attrs.html#default)
with the caveat that we use the `Default` value if the field deserialization
fails, as we don't serialize field names and there is no way to tell if it is
present or not.

For `RlpEncodable` and `RlpDecodable`, fields can also use
`#[rlp(with = path)]` to delegate encoding/decoding to helper functions:

```rust
mod compat_type {
    pub fn encode(v: &RemoteType, out: &mut dyn alloy_rlp::BufMut) { /* ... */ }
    pub fn length(v: &RemoteType) -> usize { /* ... */ }
    pub fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<RemoteType> { /* ... */ }
}

#[derive(alloy_rlp::RlpEncodable, alloy_rlp::RlpDecodable)]
struct Msg {
    #[rlp(with = compat_type)]
    remote: RemoteType,
}
```

Only the functions needed by the derived trait are required:
- `RlpEncodable` needs `path::encode` and `path::length`
- `RlpDecodable` needs `path::decode`
