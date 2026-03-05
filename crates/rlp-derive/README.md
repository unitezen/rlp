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

For `RlpEncodable`, you can also define struct-level encode hooks:

- `#[rlp(pre_encode_with = path)]`
- `#[rlp(post_encode_with = path)]`

The hook module must provide:

```rust
fn length<T>(value: &T, inner_payload_length: usize) -> usize
fn encode<T>(value: &T, inner_payload_length: usize, out: &mut dyn alloy_rlp::BufMut)
```

`inner_payload_length` is the derived struct payload size before encode hooks.
The derive adds both hook lengths into the list payload length, then calls
`pre_encode_with::encode`, encodes fields, and finally calls
`post_encode_with::encode`.

For `RlpDecodable`, you can also define struct-level decode hooks:

- `#[rlp(nolist)]` (decode fields directly, without an outer RLP list header)
- `#[rlp(pre_decode_with = path)]`
- `#[rlp(post_decode_with = path)]`

Each hook function must have the signature:

```rust
fn hook(buf: &mut &[u8]) -> alloy_rlp::Result<()>
```

If your hook rebinds `*buf` to a subslice (for envelope unwrapping), use an
explicit lifetime:

```rust
fn hook<'a>(buf: &mut &'a [u8]) -> alloy_rlp::Result<()>
```

`pre_decode_with` runs before list-header decoding and can rewrite/advance the
input buffer. `post_decode_with` runs after successful payload decode.
