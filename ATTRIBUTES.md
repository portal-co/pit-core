# PIT Documentation and Naming Attributes Specification

This document rigorously defines the standard attributes for documentation, human-readable names, and LLM-readable metadata in the Portal Interface Types (PIT) system.

## Overview

PIT attributes provide a mechanism for attaching metadata to interfaces, methods, and arguments. This specification defines a set of **reserved attribute names** for documentation purposes, ensuring consistent and interoperable metadata across tools, documentation generators, and language model integrations.

---

## Attribute Format

All attributes follow the standard PIT format:

```
[name=value]
```

Where:
- `name` is an alphanumeric identifier (may include `_`, `$`, `.`)
- `value` is a balanced bracket string (may contain nested brackets)

---

## Reserved Documentation Attributes

### Interface-Level Attributes

| Attribute Name | Description | Example |
|----------------|-------------|---------|
| `name` | Human-readable display name for the interface | `[name=User Authentication Service]` |
| `doc` | Full documentation text for the interface | `[doc=Provides authentication and authorization capabilities]` |
| `brief` | Short one-line summary (≤80 chars recommended) | `[brief=Auth service interface]` |
| `version` | Semantic version of the interface | `[version=1.2.0]` |
| `deprecated` | Deprecation notice with optional migration path | `[deprecated=Use AuthV2 instead]` |
| `since` | Version when the interface was introduced | `[since=0.1.0]` |
| `author` | Author or maintainer identifier | `[author=portal-co]` |
| `license` | License identifier (SPDX recommended) | `[license=MIT]` |
| `see` | Reference to related interfaces or documentation | `[see=AuthV2]` |
| `category` | Logical grouping category | `[category=security]` |
| `tags` | Comma-separated tags for classification | `[tags=auth,security,identity]` |

### Method-Level Attributes

| Attribute Name | Description | Example |
|----------------|-------------|---------|
| `name` | Human-readable display name for the method | `[name=Authenticate User]` |
| `doc` | Full documentation text for the method | `[doc=Validates credentials and returns a session token]` |
| `brief` | Short one-line summary (≤80 chars recommended) | `[brief=Authenticate a user]` |
| `deprecated` | Deprecation notice with optional migration path | `[deprecated=Use authenticateV2]` |
| `since` | Version when the method was introduced | `[since=0.2.0]` |
| `throws` | Comma-separated list of error conditions | `[throws=InvalidCredentials,RateLimited]` |
| `async` | Indicates asynchronous execution (`true`/`false`) | `[async=true]` |
| `idempotent` | Whether repeated calls have the same effect | `[idempotent=true]` |
| `pure` | Whether the method has no side effects | `[pure=true]` |
| `example` | Usage example (may contain code) | `[example=auth.login(user, pass)]` |

### Argument-Level Attributes

| Attribute Name | Description | Example |
|----------------|-------------|---------|
| `name` | Human-readable display name for the argument | `[name=User ID]` |
| `doc` | Full documentation text for the argument | `[doc=The unique identifier of the user]` |
| `brief` | Short one-line summary | `[brief=User identifier]` |
| `default` | Default value if not provided | `[default=0]` |
| `range` | Valid range for numeric types | `[range=0..100]` |
| `pattern` | Regex pattern for validation | `[pattern=^[a-z]+$]` |
| `unit` | Unit of measurement | `[unit=seconds]` |
| `example` | Example value | `[example=12345]` |

---

## LLM-Readable Attributes

These attributes are specifically designed to provide context for language models processing PIT interfaces:

| Attribute Name | Description | Example |
|----------------|-------------|---------|
| `llm.context` | Extended context for LLM understanding | `[llm.context=This method is called during OAuth flow]` |
| `llm.intent` | The intended use case or purpose | `[llm.intent=user authentication]` |
| `llm.constraints` | Constraints the LLM should be aware of | `[llm.constraints=Must validate input before calling]` |
| `llm.examples` | Multiple examples for few-shot learning | `[llm.examples=login(admin,secret);login(user,pass)]` |
| `llm.related` | Related concepts or interfaces | `[llm.related=session management,token refresh]` |

---

## Info File Structure

The Info file provides out-of-band metadata for interfaces, separate from the interface definition itself. This allows documentation and annotations to be stored, merged, and updated independently.

### Format

```
<hex-resource-id>: [<info-entry>]
```

Where `<info-entry>` contains:

```
root [attr1=val1]
root [attr2=val2]
method methodName [attr1=val1]
method methodName [attr2=val2]
```

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

### Example

```
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef: [
    root [name=Authentication Interface]
    root [doc=Provides user authentication and session management]
    root [version=1.0.0]
    root [category=security]
    method login [name=User Login]
    method login [doc=Authenticates a user with credentials]
    method login [brief=Log in a user]
    method logout [name=User Logout]
    method logout [doc=Terminates the current session]
]
```

---

## Merging Semantics

When multiple Info sources are merged:

1. **Interface entries** with the same resource ID are merged
2. **Attributes** with the same name are overwritten (last wins)
3. **Method entries** with the same name are merged
4. All attributes are sorted by name for deterministic output

---

## Attribute Versioning

The `docAttrVer` attribute controls the interpretation of documentation attributes:

| Version | Description |
|---------|-------------|
| `0` (default) | Basic documentation (name, doc, brief only) |
| `1` | Extended documentation (all standard attributes) |
| `2` | LLM attributes supported |

Example:
```
[docAttrVer=1][name=My Interface]{...}
```

---

## Feature Gates

In the Rust implementation (`pit-core`), documentation attribute support is gated behind the `doc-attrs` feature:

```toml
[features]
doc-attrs = []
```

When enabled, the following functionality becomes available:
- `Attr::as_name()`, `Attr::as_doc()`, `Attr::as_brief()` helper methods
- `Attr::from_name()`, `Attr::from_doc()`, `Attr::from_brief()` constructors
- `InfoEntry::name()`, `InfoEntry::doc()` convenience accessors
- `MethEntry::name()`, `MethEntry::doc()` convenience accessors

---

## Best Practices

1. **Always provide `brief`**: Keep it under 80 characters for display in IDEs
2. **Use `doc` for details**: Include full context, examples, and caveats
3. **Use `name` for display**: Provide human-readable names with proper spacing and capitalization
4. **Use `llm.*` sparingly**: Only when additional context aids LLM understanding
5. **Version your interfaces**: Use `version` and `since` for API evolution
6. **Mark deprecations early**: Use `deprecated` with migration guidance

---

## Compatibility Notes

- Unknown attributes are preserved and passed through
- Standard attributes should not conflict with user-defined attributes
- The `llm.` prefix is reserved for LLM-related attributes
- The `pit.` prefix is reserved for future PIT system attributes

---

## Examples

### Complete Interface Example

```
[docAttrVer=1]
[name=User Service]
[doc=Manages user accounts and profiles]
[brief=User management service]
[version=2.0.0]
[category=identity]
{
    getUser[name=Get User][doc=Retrieves a user by ID][pure=true](I64) -> (R~b64...~n);
    createUser[name=Create User][doc=Creates a new user account](R~b64...~) -> (I64);
    deleteUser[name=Delete User][doc=Removes a user account][deprecated=Use deactivateUser](I64) -> ()
}
```

### Info File Example

```
abcd...1234: [
    root [name=User Service]
    root [llm.context=Core identity management interface for the portal system]
    method getUser [name=Get User]
    method getUser [llm.intent=retrieve user information for display or processing]
]
```

---

For implementation details, see the doc comments in `src/lib.rs` and `src/info.rs`.
