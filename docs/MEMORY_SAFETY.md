# Adamantium memory-safety model

This document defines the memory rules implemented by the current compiler and
runtime. The model intentionally excludes raw pointers and manual allocation.

## Ownership

Every local variable owns its value. Assigning ordinary values creates a value
copy. Class and List assignments copy their outer runtime storage. Immutable
string data can be shared because Adamantium does not permit mutation of string
storage.

Runtime allocations currently remain alive until process shutdown. `remove`
invalidates a source-level name and runs lifecycle behavior, but it does not
call a raw memory deallocator. This makes double-free and physical
use-after-free impossible in the current runtime. Runtime reclamation may be
added later only if it preserves these language rules.

## Aliases

`as_variable` creates another name for the same local storage slot. Aliases are
non-owning references and cannot cross a function boundary. Writes through any
name are visible through every alias.

Removing one name leaves the slot alive while another alias exists. The
`__remove__` hook runs only when the last name for that slot is removed. A
removed name cannot be read, written, removed again, or used to create another
reference. Scalar aliases may be disconnected to create an independent copy.

## Offsets

An offset is a typed, non-owning reference to a local variable's storage slot.
It is created with `.offset` or `.get_offset()` and read with `.by_offset` or
`.value_by_offset`. Reading creates a value copy and preserves the target type.

Offsets cannot appear in function signatures, class fields, Lists, or printed
output. They therefore cannot leave the stack frame containing their target.
The compiler tracks the target slot and rejects dereferencing it after the last
name for that slot has been removed. Raw numeric addresses are never exposed.

## Objects and lifecycle hooks

Class values use runtime-managed storage that remains valid until shutdown.
`__new__` runs after field initialization, `__change__` runs after a successful
field write, and `__remove__` runs once when the final name is removed.

The compiler rejects direct field mutation from `__change__` and recursive
removal from `__remove__`. This prevents immediate recursive lifecycle calls.
Lifecycle methods cannot manually free storage or access raw addresses.

## Diagnostics

Memory-safety violations are compile errors with source locations. These
include access through removed names, invalid alias disconnection, escaped or
printed offsets, offsets stored in containers, dereferencing removed targets,
recursive lifecycle operations, and invalid field or method access.
