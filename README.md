# pit-core

Core library for portal interface types, PIT. Provides generic utilities, parsing, and data structures for use in no_std environments.

## Features
- Generic parameter mangling/demangling
- Attribute and arity management
- Interface and method info structures
- Pcode expression trees
- Utility wrappers for hashing and formatting

## Modules
- `lib.rs`: Main entry point, re-exports modules and core types
- `generics.rs`: Generic parameter handling and mangling
- `info.rs`: Interface and method metadata structures
- `pcode.rs`: Expression trees for pcode operations
- `util.rs`: Utility types for formatting and hashing

## Usage
Add `pit-core` as a dependency in your Cargo.toml:
```toml
pit-core = "*"
```

Import and use modules as needed:
```rust
use pit_core::generics::*;
use pit_core::info::*;
use pit_core::pcode::*;
```

## License
MIT

## Goals
- [ ] Add project goals

## Progress
- [ ] Initial setup

---
*AI assisted*
