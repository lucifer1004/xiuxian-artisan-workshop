# Document Identity Protocol

:PROPERTIES:
:ID: d70c0f841afe84b92272b5ef03f5a3cd2e12f46a
:TYPE: CORE
:STATUS: ACTIVE
:END:

Package-local documentation in `xiuxian-wendao-modelica/docs/` uses opaque, stable `:ID:` values instead of human-readable title slugs.

## Rule

Each document must carry a unique `:ID:` that is treated as an identity key, not as a label.

For this package-local docs tree, the current rule is:

- `:ID:` uses a hash-shaped opaque identifier
- the value should stay stable across title edits
- the value should not be derived from the visible document title

## Why

Readable title-like IDs are fragile:

- renaming a page changes the apparent identity
- two pages can drift into similar names
- downstream references start depending on presentation text

An opaque identifier keeps identity separate from:

- document title
- file path label
- section naming

## Current Package Convention

For the current `xiuxian-wendao-modelica/docs/` tree, page IDs are assigned as hash-shaped values and treated as immutable once published in the package docs.

This convention is local to the package docs tree and can later be aligned with a wider repository-level org-id policy if the rest of the workspace adopts the same approach.

## Current Enforcement Path

The current enforcement path is now Wendao-native:

- `wendao audit` can flag non-opaque top-level `:ID:` values in package-local crate docs
- `wendao fix` can surgically rewrite those top-level `:ID:` values or insert a missing `:ID:` line into the first property drawer

That enforcement currently covers in-place file remediation. It does not yet create missing docs directories or missing index pages.
