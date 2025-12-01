# PIT-Core Format Specification

This document describes the format parsed and rendered by the core library for portal interface types (PIT), as implemented in `src/lib.rs`.

## Overview
PIT defines a structured format for describing interfaces, methods, arguments, resources, and attributes. The format is designed for extensibility, annotation, and compatibility with serialization and parsing routines. It is used for portal-co projects and is suitable for no_std environments.

---

## Identifiers
- Identifiers may contain alphanumeric characters, `_`, `$`, and `.`.
- Example: `foo_bar$baz.qux`

---

## Attributes
- Attributes are key-value pairs used for metadata and annotations.
- Format: `[name=value]`
- Multiple attributes can be listed in sequence.
- Example: `[version=1][author=alice]`

---

## Arity (Generics)
- Arity describes generic parameters and their structure.
- Format: `<T <U>>` (where `T` and `U` are identifiers, and arity can be nested)
- Example: `<T <U <V>>>`

---

## Resource Types (`ResTy`)
- Represents a resource type, which may be:
  - `None`: No resource
  - `Of([u8; 32])`: A resource identified by a 32-byte ID
  - `This`: The current resource ("this")
- Rendered as:
  - `this` for `This`
  - `~b64<base64>~` for base64-encoded 32-byte ID (if `ridFmtVer >= 1`)
  - `<hex>` for hex-encoded 32-byte ID (default)
- Example: `this`, `~b64SGVsbG9Xb3JsZCE~`, `0123456789abcdef...`

---

## Arguments (`Arg`)
- Argument types for methods:
  - `I32`, `I64`, `F32`, `F64`: Primitive types
  - `Resource`: With type, nullability, ownership, and annotations
- Resource argument format:
  - `[attr1=val1][attr2=val2]R<resource>n&`
    - `n` for nullable
    - `&` for reference (not taken)
- Example: `[foo=bar]R~b64SGVsbG8~n&`

---

## Method Signatures (`Sig`)
- Format: `[attr1=val1](param1,param2) -> (ret1,ret2)`
- Parameters and return values are lists of `Arg`.
- Example: `[ver=1](I32,[x=1]RI64n) -> (F64)`

---

## Interfaces
- An interface contains methods and interface-level annotations.
- Format:
  - `[attr1=val1]{method1(sig1);method2(sig2)}`
- Example:
  - `[api=foo]{get(I32) -> (F64);set([x=1]RI64) -> ()}`

---

## Parsing and Rendering
- All major types (`Arity`, `Attr`, `ResTy`, `Arg`, `Sig`, `Interface`) have both parsing and rendering routines.
- Parsing functions accept a string and return the corresponding type and remaining input.
- Rendering routines produce the canonical string representation, suitable for hashing and serialization.

---

## Notes
- Attributes and annotations are extensible and can be used for versioning, ABI compatibility, and metadata.
- Resource IDs can be rendered in base64 or hex, depending on attribute `ridFmtVer`.
- The format is designed for deterministic serialization and parsing.

---

## Info File Structure

The Info file provides out-of-band metadata for interfaces, allowing documentation and annotations to be stored separately from interface definitions. This enables metadata to be merged, updated, and managed independently.

### Format Overview

An Info file consists of one or more interface entries:

```
<hex-resource-id>: [<info-entry>]
```

Where:
- `<hex-resource-id>` is a 64-character hex string (32 bytes)
- `<info-entry>` contains root and method attributes

### Info Entry Structure

```
root [attr1=val1]
root [attr2=val2]
method methodName [attr1=val1]
method methodName [attr2=val2]
```

- `root` attributes apply to the interface itself
- `method` attributes apply to the named method

### Grammar

```ebnf
info         = { info-item } ;
info-item    = hex64, ":", "[", entry, "]" ;
hex64        = 64 * hex-digit ;
entry        = { root-attr }, { method-attr } ;
root-attr    = "root", attribute ;
method-attr  = "method", identifier, attribute ;
attribute    = "[", name, "=", value, "]" ;
```

### Info Structures

- **Info**: Top-level container, maps resource IDs to `InfoEntry`
- **InfoEntry**: Contains interface-level attributes and method entries
- **MethEntry**: Contains method-level attributes

### Merging

When multiple Info sources are merged:
1. Interface entries with the same resource ID are merged
2. Attributes with the same name are overwritten (last wins)
3. Method entries with the same name are merged
4. All attributes are sorted by name for deterministic output

### Example

```
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef: [
    root [name=Authentication Service]
    root [doc=Provides user authentication]
    root [version=1.0.0]
    method login [name=User Login]
    method login [doc=Authenticates a user with credentials]
    method logout [name=User Logout]
    method logout [doc=Terminates the current session]
]
```

---

## Documentation Attributes

PIT supports a set of reserved attributes for documentation and metadata. These are fully documented in `ATTRIBUTES.md`.

### Common Documentation Attributes

| Attribute | Description |
|-----------|-------------|
| `name` | Human-readable display name |
| `doc` | Full documentation text |
| `brief` | Short one-line summary |
| `deprecated` | Deprecation notice |
| `since` | Version introduced |
| `category` | Logical grouping |

### LLM-Readable Attributes

| Attribute | Description |
|-----------|-------------|
| `llm.context` | Extended context for LLM understanding |
| `llm.intent` | Intended use case or purpose |
| `llm.constraints` | Constraints for LLM processing |

### Feature Gate

In the Rust implementation, documentation attribute helpers are gated behind the `doc-attrs` feature:

```toml
[features]
doc-attrs = []
```

---

## Example
```
[api=example]{
    get([ver=1]RI32n) -> (F64);
    set([x=1]RI64) -> ()
}
```

---

For further details, see the doc comments and parsing/rendering implementations in `src/lib.rs`.
For documentation attributes, see `ATTRIBUTES.md`.
