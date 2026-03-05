//! Tests for the derive macros.

#![cfg(feature = "derive")]
#![allow(dead_code)]

use alloy_rlp::*;

#[test]
fn simple_derive() {
    #[derive(RlpEncodable, RlpDecodable, RlpMaxEncodedLen, PartialEq, Debug)]
    struct MyThing(#[rlp] [u8; 12]);

    let thing = MyThing([0; 12]);

    // roundtrip fidelity
    let mut buf = Vec::new();
    thing.encode(&mut buf);
    let decoded = MyThing::decode(&mut buf.as_slice()).unwrap();
    assert_eq!(thing, decoded);

    // does not panic on short input
    assert_eq!(Err(Error::InputTooShort), MyThing::decode(&mut [0x8c; 11].as_ref()))
}

#[test]
const fn wrapper() {
    #[derive(RlpEncodableWrapper, RlpDecodableWrapper, RlpMaxEncodedLen, PartialEq, Debug)]
    struct Wrapper([u8; 8]);

    #[derive(RlpEncodableWrapper, RlpDecodableWrapper, PartialEq, Debug)]
    struct ConstWrapper<const N: usize>([u8; N]);
}

#[test]
const fn generics() {
    trait LT<'a> {}

    #[derive(RlpEncodable, RlpDecodable, RlpMaxEncodedLen)]
    struct Generic<T, U: for<'a> LT<'a>, V: Default, const N: usize>(T, usize, U, V, [u8; N])
    where
        U: std::fmt::Display;

    #[derive(RlpEncodableWrapper, RlpDecodableWrapper, RlpMaxEncodedLen)]
    struct GenericWrapper<T>(T)
    where
        T: Sized;
}

#[test]
const fn opt() {
    #[derive(RlpEncodable, RlpDecodable)]
    #[rlp(trailing)]
    struct Options<T>(Option<Vec<T>>);

    #[derive(RlpEncodable, RlpDecodable)]
    #[rlp(trailing)]
    struct Options2<T> {
        a: Option<T>,
        #[rlp(default)]
        b: Option<T>,
    }
}

/// Test that multiple attributes can be combined in a single `#[rlp(...)]`.
/// See <https://github.com/alloy-rs/rlp/issues/9>
#[test]
fn multiple_attrs_combined() {
    /// A type that intentionally does NOT implement `Encodable` or `Decodable`.
    /// This verifies that `#[rlp(default, skip)]` works for such types.
    #[derive(PartialEq, Debug, Default)]
    struct Cache(u64);

    // Test `#[rlp(default, skip)]` order
    #[derive(RlpEncodable, RlpDecodable, PartialEq, Debug)]
    struct Foo {
        pub bar: u64,
        #[rlp(default, skip)]
        pub cache: Cache,
    }

    let foo = Foo { bar: 42, cache: Cache(123) };

    let mut buf = Vec::new();
    foo.encode(&mut buf);

    let decoded = Foo::decode(&mut buf.as_slice()).unwrap();
    assert_eq!(decoded.bar, 42);
    assert_eq!(decoded.cache, Cache::default());

    // Test `#[rlp(skip, default)]` reverse order
    #[derive(RlpEncodable, RlpDecodable, PartialEq, Debug)]
    struct Bar {
        pub baz: u64,
        #[rlp(skip, default)]
        pub cache: Cache,
    }

    let bar = Bar { baz: 99, cache: Cache(456) };

    let mut buf2 = Vec::new();
    bar.encode(&mut buf2);

    let decoded2 = Bar::decode(&mut buf2.as_slice()).unwrap();
    assert_eq!(decoded2.baz, 99);
    assert_eq!(decoded2.cache, Cache::default());
}

/// Test that `#[rlp(skip)]` alone works.
#[test]
fn skip_field() {
    #[derive(PartialEq, Debug, Default)]
    struct NotEncodable(u64);

    #[derive(RlpEncodable, PartialEq, Debug)]
    struct WithSkip {
        pub value: u64,
        #[rlp(skip)]
        pub skipped: NotEncodable,
    }

    let s = WithSkip { value: 42, skipped: NotEncodable(123) };

    let mut buf = Vec::new();
    s.encode(&mut buf);

    // Decode as a struct without the skipped field to verify it wasn't encoded
    #[derive(RlpDecodable, PartialEq, Debug)]
    struct WithoutSkip {
        pub value: u64,
    }

    let decoded = WithoutSkip::decode(&mut buf.as_slice()).unwrap();
    assert_eq!(decoded.value, 42);
}

#[test]
fn with_attr_roundtrip() {
    mod compat_type {
        #[derive(Clone, PartialEq, Debug)]
        pub(super) struct RemoteType(pub u64);

        pub(super) fn encode(v: &RemoteType, out: &mut dyn alloy_rlp::BufMut) {
            alloy_rlp::Encodable::encode(&v.0, out);
        }

        pub(super) fn length(v: &RemoteType) -> usize {
            alloy_rlp::Encodable::length(&v.0)
        }

        pub(super) fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<RemoteType> {
            Ok(RemoteType(alloy_rlp::Decodable::decode(buf)?))
        }
    }

    #[derive(RlpEncodable, RlpDecodable, PartialEq, Debug)]
    struct Msg {
        #[rlp(with = compat_type)]
        remote: compat_type::RemoteType,
    }

    let msg = Msg { remote: compat_type::RemoteType(7) };
    let mut buf = Vec::new();
    msg.encode(&mut buf);
    let decoded = Msg::decode(&mut buf.as_slice()).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn with_attr_encodable_only() {
    mod compat_type {
        #[derive(Clone, PartialEq, Debug)]
        pub(super) struct RemoteType(pub u64);

        pub(super) fn encode(v: &RemoteType, out: &mut dyn alloy_rlp::BufMut) {
            alloy_rlp::Encodable::encode(&v.0, out);
        }

        pub(super) fn length(v: &RemoteType) -> usize {
            alloy_rlp::Encodable::length(&v.0)
        }
    }

    #[derive(RlpEncodable)]
    struct Msg {
        #[rlp(with = compat_type)]
        remote: compat_type::RemoteType,
    }

    let msg = Msg { remote: compat_type::RemoteType(7) };
    let mut buf = Vec::new();
    msg.encode(&mut buf);
    assert_eq!(buf, vec![0xc1, 0x07]);
}

#[test]
fn with_attr_decodable_only() {
    mod compat_type {
        #[derive(Clone, PartialEq, Debug)]
        pub(super) struct RemoteType(pub u64);

        pub(super) fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<RemoteType> {
            Ok(RemoteType(alloy_rlp::Decodable::decode(buf)?))
        }
    }

    #[derive(RlpDecodable, PartialEq, Debug)]
    struct Msg {
        #[rlp(with = compat_type)]
        remote: compat_type::RemoteType,
    }

    let mut input = [0xc1, 0x07].as_slice();
    let decoded = Msg::decode(&mut input).unwrap();
    assert_eq!(decoded, Msg { remote: compat_type::RemoteType(7) });
}

#[test]
fn pre_decode_with_hook() {
    #[derive(RlpDecodable, PartialEq, Debug)]
    #[rlp(pre_decode_with = unwrap_outer)]
    struct Msg {
        value: u64,
    }

    fn unwrap_outer<'a>(buf: &mut &'a [u8]) -> alloy_rlp::Result<()> {
        let outer_payload = alloy_rlp::Header::decode_bytes(buf, true)?;
        let mut payload_cursor = outer_payload;
        let inner_header = alloy_rlp::Header::decode(&mut payload_cursor)?;
        if !inner_header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        let inner_len = inner_header.length_with_payload();
        if outer_payload.len() != inner_len {
            return Err(alloy_rlp::Error::Custom("invalid nested envelope"));
        }
        *buf = &outer_payload[..inner_len];
        Ok(())
    }

    let mut input = [0xc2, 0xc1, 0x07].as_slice();
    let decoded = Msg::decode(&mut input).unwrap();
    assert_eq!(decoded, Msg { value: 7 });
}

#[test]
fn post_decode_with_hook() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static POST_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[derive(RlpDecodable, PartialEq, Debug)]
    #[rlp(post_decode_with = post_check)]
    struct Msg {
        value: u64,
    }

    fn post_check(buf: &mut &[u8]) -> alloy_rlp::Result<()> {
        if !buf.is_empty() {
            return Err(alloy_rlp::Error::Custom("post hook expected empty buffer"));
        }
        POST_HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    let mut input = [0xc1, 0x07].as_slice();
    let decoded = Msg::decode(&mut input).unwrap();
    assert_eq!(decoded, Msg { value: 7 });
    assert_eq!(POST_HOOK_CALLS.load(Ordering::SeqCst), 1);
}
