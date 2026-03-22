# pit-core

Core library for Portal Interface Types (PIT). Provides data structures, a parser, a renderer, and SHA3-256 hashing routines that define the PIT wire format. Targets `no_std` environments with `alloc`.

**Version:** 0.5.0-alpha.1
**License:** CC0-1.0
**Edition:** Rust 2024

---

## What is PIT?

PIT is a compact text format for describing typed interfaces: their methods, argument types, resource references, and key-value metadata attributes. An interface is serialized to a canonical string via its `Display` implementation, which is then SHA3-256 hashed to produce a stable 32-byte resource ID (RID). This RID is how interfaces are referenced within the system.

---

## Wire Format

### Identifiers

Alphanumeric characters plus `_`, `$`, `.`. Parsed by `ident()`. Example: `foo_bar$baz.qux`.

### Attributes

Key-value pairs in the form `[name=value]`. Attribute values support nested brackets (balanced by depth). Multiple attributes can appear in sequence. Attribute lists are sorted by name for deterministic output.

```
[version=1][author=alice]
```

The `name` in an attribute follows the same character set as identifiers. The `value` may contain balanced nested brackets; the parser counts `[`/`]` depth to find the closing `]`.

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

Resource identifiers inside `R<...>` can be:
- `this` — the current resource (`ResTy::This`)
- `~b64<base64>~` — 32-byte ID base64-encoded with no padding (when `ridFmtVer >= 1` on the enclosing interface)
- `<64 hex chars>` — 32-byte ID hex-encoded (default; used when `ridFmtVer` is absent or 0)
- _(nothing)_ — `ResTy::None`

Arguments may carry their own attribute annotations placed before the type token.

### Method signatures

```
[attr...](param, param, ...) -> (ret, ret, ...)
```

Both parameter and return lists use comma-separated `Arg` values. Whitespace around `->` and inside `(...)` is accepted during parsing and stripped on output.

### Interfaces

```
[attr...]{methodName[attr...](params) -> (rets); ...}
```

Methods are stored in a `BTreeMap` and rendered in sorted (lexicographic) order, separated by `;`. Interface-level attributes appear before the `{`. Example:

```
[api=foo]{get([ver=1]RI32n) -> (F64);set([x=1]RI64) -> ()}
```

### Arity (generic parameter structure)

Arity is a recursive structure describing generic parameters: `<name arity name arity ...>`. Parsed by `Arity::parse`. Display format uses the same `<...>` syntax. Used to represent generic parameter shape, not concrete values.

### Resource IDs

`Interface::rid()` formats the interface via its `Display` impl (which reads `ridFmtVer` from the interface's own annotations to select hex vs. base64 resource encoding), streams the resulting UTF-8 bytes through `WriteUpdate` into a `Sha3_256` hasher without a heap allocation, and returns `[u8; 32]`. `Interface::rid_str()` returns the hex-encoded form.

Note: the standalone `Display` impls for `Sig`, `Arg`, `ArgTy`, and `ResTy` always use hex resource encoding (`gattrs` returns `None` for all keys). Only `Interface::fmt` passes its own `ann` vector as the attribute resolver, making `ridFmtVer` effective.

---

## Crate Structure

### `lib.rs` — core types and parsers

#### Free functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `ident` | `(&str) -> IResult<&str, &str>` | Parse an identifier (alphanum + `_$.\`) |
| `parse_balanced` | `(&str) -> IResult<&str, String>` | Parse a bracket-balanced string value (stops before the unmatched `]`) |
| `parse_attr` | `(&str) -> IResult<&str, Attr>` | Parse one `[name=value]` attribute |
| `parse_attrs` | `(&str) -> IResult<&str, Vec<Attr>>` | Parse zero or more attributes; result is sorted by name |
| `parse_resty` | `(&str) -> IResult<&str, ResTy>` | Parse a `ResTy` (`this`, `~b64...~`, 64 hex chars, or empty → `None`) |
| `parse_arg` | `(&str) -> IResult<&str, Arg>` | Parse an `Arg` (optional leading attributes then type token) |
| `parse_sig` | `(&str) -> IResult<&str, Sig>` | Parse a `Sig` |
| `parse_interface` | `(&str) -> IResult<&str, Interface>` | Parse an `Interface` |
| `merge` | `(Vec<Attr>, Vec<Attr>) -> Vec<Attr>` | Merge two attribute lists; last-wins by name, result sorted |
| `retuple` | `(Vec<Arg>) -> Interface` | Wrap args as methods named `v0`, `v1`, … each with no params and one return value |

#### Types

**`Attr`** — `{ name: String, value: String }`. Derives `Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug`. Implements `Display` as `[name=value]`.

Unconditional methods:
- `as_wasm_abi(&self) -> Option<usize>` — reads `wasmAbiVer`; value stored as 0-based hex, returned as 1-based
- `from_wasm_abi(ver: usize) -> Option<Self>` — constructs `wasmAbiVer` attr (ver 0 → `None`)
- `as_ver(&self, key: &str) -> Option<usize>` — generic 0-based-hex-to-1-based-usize reader
- `from_ver(ver: usize, key: &str) -> Option<Self>` — generic 1-based-usize-to-0-based-hex writer

Feature-gated `doc-attrs` methods (see Features section below).

---

**`Arity`** — `{ to_fill: BTreeMap<String, Arity> }`. Recursive generic parameter structure. Derives `Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default`. Implements `Display` and `Arity::parse`.

Methods:
- `is_simple(&self, depth: usize) -> bool` — returns `false` for `depth == 0`; otherwise `true` iff all children are `is_simple(depth - 1)`

---

**`ResTy`** — `#[non_exhaustive]` enum. Variants: `None`, `Of([u8; 32])`, `This`. Derives `Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug`. `Display` always uses hex encoding (no attribute context); `render()` uses base64 when `ridFmtVer >= 1`.

---

**`ArgTy`** — `#[non_exhaustive]` enum. Variants: `I32`, `I64`, `F32`, `F64`, `Resource { ty: ResTy, nullable: bool, take: bool }`. `take: true` means owned (no `&` suffix); `take: false` means borrowed (`&` suffix). `Display` always uses hex encoding.

Methods on `ArgTy`:
- `with_attrs(self, ann: Vec<Attr>) -> Arg`
- `into_arg(self) -> Arg`

---

**`Arg`** — `{ ty: ArgTy, ann: Vec<Attr> }`. Renders its `ann` before the type token. `Display` always uses hex encoding.

Constructors:
- `Arg::new(ty: ArgTy) -> Self`
- `Arg::with_attrs(ty: ArgTy, ann: Vec<Attr>) -> Self`
- `Arg::i32()`, `Arg::i64()`, `Arg::f32()`, `Arg::f64()`
- `Arg::resource(ty: ResTy, nullable: bool, take: bool) -> Self`

Builder method:
- `Arg::with_attr(self, attr: Attr) -> Self` — appends and re-sorts `ann` by name

---

**`Sig`** — `{ ann: Vec<Attr>, params: Vec<Arg>, rets: Vec<Arg> }`. Derives `Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug`.

---

**`Interface`** — `{ methods: BTreeMap<String, Sig>, ann: Vec<Attr> }`. Derives `Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug`. `Display` resolves `ridFmtVer` from `self.ann`.

Methods:
- `rid(&self) -> [u8; 32]` — SHA3-256 of the canonical display string, streamed without heap allocation via `WriteUpdate`
- `rid_str(&self) -> String` — hex-encoded RID

---

### `info.rs` — out-of-band metadata

An Info file attaches documentation and annotations to interfaces identified by their RID, independently of the interface definition itself.

#### Text format

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

Lines inside the brackets start with one of four keywords:
- `root` — attribute applied to the interface itself
- `method <name>` — attribute applied to the named method
- `param <method> <index>` — attribute applied to the parameter at the given 0-based index
- `return <method> <index>` — attribute applied to the return value at the given 0-based index

Note: `method`, `param`, and `return` use `alphanumeric1` for name parsing (letters and digits only, no `_$.\`), which is narrower than the `ident` parser used for interface method names.

#### Types

| Type | Fields | Description |
|------|--------|-------------|
| `Info` | `interfaces: BTreeMap<[u8; 32], InfoEntry>` | Top-level container |
| `InfoEntry` | `attrs: Vec<Attr>, methods: BTreeMap<String, MethEntry>` | Per-interface annotations |
| `MethEntry` | `attrs: Vec<Attr>, params: BTreeMap<usize, ParamEntry>, returns: BTreeMap<usize, ParamEntry>` | Per-method annotations with indexed parameter and return entries |
| `ParamEntry` | `attrs: Vec<Attr>` | Per-parameter/return annotations |

All four types implement `Display`, a `merge()` method (by-name last-wins, sorted), and `parse()`. Legacy free functions `parse_entry` and `parse_info` delegate to `InfoEntry::parse` and `Info::parse` respectively.

`MethEntry` additional methods: `param(index)`, `return_value(index)`, `add_param`, `add_return`, `add_param_attr`, `add_return_attr`.

Merging semantics: same-RID `InfoEntry`s are merged; same-key attrs are overwritten (last wins); method entries with the same name are recursively merged; same-index param/return entries are recursively merged; all attr lists are sorted by name.

---

### `generics.rs` — generic parameter mangling (unstable)

Gated behind the `unstable-generics` feature. The public module is declared `#[instability::unstable(feature = "generics")]`.

| Item | Description |
|------|-------------|
| `ARITY_KEY` | `"generic_params.modern"` |
| `GENERIC_KEY` | `"generics.modern"` |
| `Mangle` trait | `demangle(&str) -> IResult<&str, Self>` + `mangle(&self, &mut Formatter)` |
| `Mangled<'a>` | `#[repr(transparent)]` `Display` wrapper over `&dyn Mangle` |
| `Arity: Mangle` | Encodes count as `;N` followed by `P<name>` entries recursively (stack-based decode) |
| `Param` | `#[non_exhaustive]` enum: `Attr(Attr)`, `Interface { rid: [u8;32], params: BTreeMap<String,Param> }`, `Param { param: String, nest: BTreeMap<String,Param> }` |
| `Param: Mangle` | `Attr` → `[k=v]`; `Interface` → `R<hex64>;<N>;<key>;<param>…`; `Param` → `$<name>;<N>;<key>;<param>…` |

---

### `pcode.rs` — expression trees (unstable)

Gated behind the `unstable-pcode` feature. The public module is declared `#[instability::unstable(feature = "pcode")]`.

Data types only — no parser or evaluator is implemented.

| Type | Description |
|------|-------------|
| `PExpr` | `#[non_exhaustive]` enum: `Param(usize)`, `Var(String)`, `Call { rid, method, obj, args, ret }`, `LitI32(u32)`, `LitI64(u64)`, `LitF32(u32)`, `LitF64(u64)` |
| `Pat` | `{ params: Vec<String>, body: Box<PExpr> }` — a pattern with named parameters and a body expression |

Float literals are stored as IEEE 754 bit patterns in `u32`/`u64`.

---

### `util.rs`

**`WriteUpdate<'a, 'b>`** — bridges `core::fmt::Write` to `sha3::digest::Update`. Holds `wrapped: &'a mut (dyn Update + 'b)`. The `write_str` implementation calls `self.wrapped.update(s.as_bytes())` and always returns `Ok(())`. Used by `Interface::rid()` to stream the canonical interface string into a SHA3-256 hasher without an intermediate heap allocation. Also re-exports `core` as a public item.

---

## Features

| Feature | Effect |
|---------|--------|
| `unstable-pcode` | Exposes `pub mod pcode` (gated by `#[instability::unstable]`) |
| `unstable-generics` | Exposes `pub mod generics` (gated by `#[instability::unstable]`) |
| `doc-attrs` | Enables documentation attribute helpers on `Attr`, `InfoEntry`, `MethEntry`, `ParamEntry` |

### `doc-attrs` detail

When enabled, `Attr` gains:

`as_name()`, `from_name()`, `as_doc()`, `from_doc()`, `as_brief()`, `from_brief()`, `as_deprecated()`, `from_deprecated()`, `as_llm_context()`, `from_llm_context()`, `as_llm_intent()`, `from_llm_intent()`, `as_category()`, `from_category()`, `as_since()`, `from_since()`, `as_attr(name)`, `from_attr(name, value)`.

`InfoEntry`, `MethEntry`, and `ParamEntry` each gain `name()`, `doc()`, `brief()`, `deprecated()`, `llm_context()`, `llm_intent()`, `category()`, `since()`, and `get_attr(name)` via the `impl_doc_attrs!` macro defined in `info.rs`. All these methods search the type's `attrs` field using the corresponding `Attr::as_*` methods.

---

## Reserved attribute names

See `ATTRIBUTES.md` for the full specification. Key names:

- **Documentation:** `name`, `doc`, `brief`, `deprecated`, `since`, `version`, `author`, `see`, `category`, `tags`
- **Method-level:** `throws`, `async`, `idempotent`, `pure`, `example`
- **Argument-level:** `default`, `range`, `pattern`, `unit`, `example`
- **LLM-readable:** `llm.context`, `llm.intent`, `llm.constraints`, `llm.examples`, `llm.related`
- **ABI:** `wasmAbiVer`, `ridFmtVer`, `docAttrVer`

The prefixes `llm.` and `pit.` are reserved for future use.

---

## Dependencies

| Crate | Version | Features used | Purpose |
|-------|---------|---------------|---------|
| `nom` | 8 | `alloc` | Parser combinators |
| `sha3` | 0.10.8 | _(none)_ | SHA3-256 for RID computation |
| `base64` | 0.22.1 | `alloc` | Base64 encoding/decoding for `ridFmtVer >= 1` resource IDs |
| `hex` | 0.4.3 | `alloc` | Hex encoding/decoding for resource IDs and RIDs |
| `derive_more` | 2 | `display` | `#[derive(Display)]` |
| `instability` | 0.3.7 | _(default)_ | `#[unstable]` attribute for feature-gated modules |

All dependencies use `default-features = false` except `instability`. The crate is `#![no_std]` + `extern crate alloc`.

---

## Usage

```toml
[dependencies]
pit-core = "0.5.0-alpha.1"

# With documentation attribute helpers
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

// Parse and merge info files
use pit_core::info::{Info, InfoEntry};
let (_, info) = Info::parse("deadbeef...64hex...: [\n    root [name=MyService]\n]").unwrap();
```

---

## Specification

The full format grammar and semantics are documented in `SPEC.md`. Documentation attribute conventions are in `ATTRIBUTES.md`.
