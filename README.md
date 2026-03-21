# pit-core

Core library for Portal Interface Types (PIT). Provides the data structures, parser, renderer, and hashing routines that define the PIT wire format. Designed for `no_std` environments with `alloc`.

**Version:** 0.5.0-alpha.1
**License:** CC0-1.0
**Edition:** Rust 2024

---

## What is PIT?

PIT is a compact text format for describing typed interfaces: their methods, argument types, resource references, and key-value metadata attributes. An interface is serialized to a canonical string, which is then SHA3-256 hashed to produce a stable 32-byte resource ID (RID). This RID is how interfaces are referenced within the system.

---

## Format

### Identifiers

Alphanumeric characters plus `_`, `$`, `.`. Example: `foo_bar$baz.qux`.

### Attributes

Key-value pairs: `[name=value]`. Values support nested brackets (balanced). Multiple attributes can appear in sequence. Attribute lists are sorted by name for deterministic output.

### Argument types

| Syntax | Meaning |
|--------|---------|
| `I32` | 32-bit integer |
| `I64` | 64-bit integer |
| `F32` | 32-bit float |
| `F64` | 64-bit float |
| `R<res>` | Resource (taken/owned) |
| `R<res>&` | Resource (borrowed, not taken) |
| `R<res>n` | Nullable resource (taken) |
| `R<res>n&` | Nullable resource (borrowed) |

Resource identifiers inside `R` can be:
- `this` — the current resource
- `~b64<base64>~` — 32-byte ID in base64 (when `ridFmtVer >= 1`)
- `<64 hex chars>` — 32-byte ID in hex (default)

Arguments may carry their own attribute annotations before the type token.

### Signatures

```
[attr...](param, param, ...) -> (ret, ret, ...)
```

### Interfaces

```
[attr...]{methodName[attr...](params) -> (rets); ...}
```

Methods are stored and rendered in sorted (BTreeMap) order. Example:

```
[api=foo]{get([ver=1]RI32n) -> (F64);set([x=1]RI64) -> ()}
```

### Resource IDs

`Interface::rid()` serializes the interface to its canonical `Display` string, feeds it through SHA3-256 via the `WriteUpdate` adapter, and returns `[u8; 32]`. `Interface::rid_str()` returns the hex-encoded form.

---

## Crate Structure

### `lib.rs` — core types and parsers (always available)

| Item | Description |
|------|-------------|
| `ident(a)` | Parse an identifier |
| `parse_balanced(a)` | Parse a bracket-balanced string value |
| `parse_attr(a)` | Parse one `[name=value]` attribute |
| `parse_attrs(a)` | Parse a sorted list of attributes |
| `parse_resty(a)` | Parse a `ResTy` |
| `parse_arg(a)` | Parse an `Arg` (with optional leading attributes) |
| `parse_sig(a)` | Parse a `Sig` |
| `parse_interface(a)` | Parse an `Interface` |
| `merge(a, b)` | Merge two attribute lists (last-wins by name, sorted) |
| `retuple(args)` | Wrap a list of `Arg` into an `Interface` with synthetic method names `v0`, `v1`, … |
| `Attr` | `{ name: String, value: String }` |
| `Arity` | Nested generic parameter structure (`BTreeMap<String, Arity>`) |
| `ResTy` | `None \| Of([u8; 32]) \| This` |
| `ArgTy` | `I32 \| I64 \| F32 \| F64 \| Resource { ty, nullable, take }` |
| `Arg` | `{ ty: ArgTy, ann: Vec<Attr> }` |
| `Sig` | `{ ann, params, rets }` |
| `Interface` | `{ methods: BTreeMap<String, Sig>, ann: Vec<Attr> }` |

`Attr` has unconditional helpers for `wasmAbiVer` (ABI version encoded as 1-based hex) and a generic `as_ver`/`from_ver` pair. All other doc-oriented helpers are behind `doc-attrs`.

### `info.rs` — out-of-band metadata (always available)

An Info file attaches documentation and annotations to interfaces identified by their RID, independently of the interface definition itself.

**Text format:**

```
<64-hex-RID>: [
    root [name=My Interface]
    root [doc=Description]
    method login [name=User Login]
    method login [doc=Authenticates a user]
    param login 0 [name=username]
    param login 0 [doc=The user's login name]
    return login 0 [name=token]
]
```

Lines start with `root`, `method <name>`, `param <method> <index>`, or `return <method> <index>`.

| Type | Description |
|------|-------------|
| `Info` | `{ interfaces: BTreeMap<[u8; 32], InfoEntry> }` |
| `InfoEntry` | `{ attrs: Vec<Attr>, methods: BTreeMap<String, MethEntry> }` |
| `MethEntry` | `{ attrs: Vec<Attr>, params: BTreeMap<usize, ParamEntry>, returns: BTreeMap<usize, ParamEntry> }` |
| `ParamEntry` | `{ attrs: Vec<Attr> }` |

All four types implement `Display`, `merge()`, and `parse()`. Merging is additive: same-RID entries merge, same-key attributes are overwritten (last wins), and attribute lists are sorted by name.

`parse_entry` and `parse_info` are available as free functions for backward compatibility.

### `generics.rs` — generic parameter mangling (unstable)

Gated behind the `unstable-generics` feature and marked `#[instability::unstable(feature = "generics")]`.

| Item | Description |
|------|-------------|
| `ARITY_KEY` | `"generic_params.modern"` |
| `GENERIC_KEY` | `"generics.modern"` |
| `Mangle` trait | `demangle(&str) -> IResult<&str, Self>` + `mangle(&self, &mut Formatter)` |
| `Mangled<'a>` | `Display` wrapper over `&dyn Mangle` |
| `Arity: Mangle` | Encodes arity as `;N` (count) followed by `P<name>` entries recursively |
| `Param` | `Attr(Attr) \| Interface { rid, params } \| Param { param, nest }` |
| `Param: Mangle` | Attrs → `[k=v]`; Interface → `R<hex64>;<N>;<key>;<param>…`; Param → `$<name>;<N>;<key>;<param>…` |

### `pcode.rs` — expression trees (unstable)

Gated behind the `unstable-pcode` feature and marked `#[instability::unstable(feature = "pcode")]`.

Contains `PExpr` and `Pat` — an expression tree for pcode operations. Currently only the data types are defined; no parser or evaluator is present.

| Variant | Description |
|---------|-------------|
| `PExpr::Param(usize)` | Reference to a numbered parameter |
| `PExpr::Var(String)` | Named variable |
| `PExpr::Call { rid, method, obj, args, ret }` | Method call on a resource |
| `PExpr::LitI32(u32)` | i32 literal (stored as u32 bits) |
| `PExpr::LitI64(u64)` | i64 literal |
| `PExpr::LitF32(u32)` | f32 literal (stored as IEEE 754 bits) |
| `PExpr::LitF64(u64)` | f64 literal (stored as IEEE 754 bits) |

`Pat { params: Vec<String>, body: Box<PExpr> }` represents a pattern with named parameters and a body expression.

### `util.rs`

`WriteUpdate<'a, 'b>` — bridges `core::fmt::Write` to `sha3::digest::Update`. Used internally by `Interface::rid()` to stream the canonical interface string into a SHA3-256 hasher without an intermediate heap allocation.

---

## Features

| Feature | Effect |
|---------|--------|
| `unstable-pcode` | Exposes `pub mod pcode` |
| `unstable-generics` | Exposes `pub mod generics` |
| `doc-attrs` | Enables doc-attribute helpers on `Attr`, `InfoEntry`, `MethEntry`, `ParamEntry` |

### `doc-attrs` detail

When enabled, `Attr` gains `as_name()`, `from_name()`, `as_doc()`, `from_doc()`, `as_brief()`, `from_brief()`, `as_deprecated()`, `from_deprecated()`, `as_llm_context()`, `from_llm_context()`, `as_llm_intent()`, `from_llm_intent()`, `as_category()`, `from_category()`, `as_since()`, `from_since()`, `as_attr(name)`, and `from_attr(name, value)`.

`InfoEntry`, `MethEntry`, and `ParamEntry` each gain `name()`, `doc()`, `brief()`, `deprecated()`, `llm_context()`, `llm_intent()`, `category()`, `since()`, and `get_attr(name)` via the `impl_doc_attrs!` macro in `info.rs`.

---

## Reserved attribute names

See `ATTRIBUTES.md` for the full specification. Key names:

- **Documentation:** `name`, `doc`, `brief`, `deprecated`, `since`, `version`, `author`, `see`, `category`, `tags`
- **Method-level:** `throws`, `async`, `idempotent`, `pure`, `example`
- **Argument-level:** `default`, `range`, `pattern`, `unit`, `example`
- **LLM-readable:** `llm.context`, `llm.intent`, `llm.constraints`, `llm.examples`, `llm.related`
- **ABI:** `wasmAbiVer`, `ridFmtVer`, `docAttrVer`

Prefixes `llm.` and `pit.` are reserved.

---

## Dependencies

| Crate | Usage |
|-------|-------|
| `nom 8` | Parser combinators (alloc, no_std) |
| `sha3 0.10` | SHA3-256 hashing for RID computation |
| `base64 0.22` | Base64 encoding/decoding for `ridFmtVer >= 1` resource IDs |
| `hex 0.4` | Hex encoding/decoding for resource IDs and RIDs |
| `derive_more 2` | `Display` derive |
| `instability 0.3` | `#[unstable]` attribute for feature-gated modules |

All dependencies are configured `default-features = false` with minimal feature sets. The crate is `no_std` + `alloc`.

---

## Usage

```toml
[dependencies]
pit-core = "0.5.0-alpha.1"

# Optional features
pit-core = { version = "0.5.0-alpha.1", features = ["doc-attrs"] }
```

```rust
use pit_core::{parse_interface, Interface, Arg, ArgTy, ResTy, Sig, Attr};
use alloc::{collections::BTreeMap, vec};

// Parse an interface from its canonical text form
let (_, iface) = parse_interface("[api=example]{get(I32) -> (F64)}").unwrap();

// Compute the interface's 32-byte resource ID
let rid: [u8; 32] = iface.rid();
let rid_hex: alloc::string::String = iface.rid_str();

// Build an interface programmatically
let mut methods = BTreeMap::new();
methods.insert("ping".into(), Sig {
    ann: vec![],
    params: vec![Arg::i32()],
    rets: vec![Arg::i64()],
});
let iface = Interface { methods, ann: vec![] };
```

---

## Specification

The full format grammar and semantics are documented in `SPEC.md`. Documentation attribute conventions are in `ATTRIBUTES.md`.
