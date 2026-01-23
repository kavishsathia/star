# star

a wasm-based runtime with garbage collection, consisting of three separate wasm modules that work together.

## architecture

| module | purpose |
|--------|---------|
| **alloc** | slab allocator for fixed-size structs |
| **dalloc** | dynamic allocator for variable-size data (lists, strings) |
| **shadow** | shadow stack + mark-and-sweep gc coordinator |

each module compiles to a separate wasm module (`cdylib`) with its own linear memory.

## memory layout

### alloc (struct memory)

- uses slab allocation with 32-block slabs per type
- header: 8 bytes (type id + mark bit)
- type table at address 12, with 16-byte records: `[size, free_ptr, struct_count, list_count]`

### dalloc (list/string memory)

- uses first-fit allocation with coalescing
- header: 20 bytes (type, mark, size, length, footer)
- types: 1 = int list, 2 = char list (string), 3 = nested list

### shadow (stack memory)

- shadow stack for gc root tracking
- each entry: 8 bytes (type + value)
- type 0 = primitive, type 1 = alloc pointer, type 2 = dalloc pointer

## gc design

the gc lives in shadow because it needs access to both alloc and dalloc memory spaces. moving it to either allocator would create circular dependencies.

flow:
1. `mark()` - traverses shadow stack, marks reachable objects in both memories
2. `sweep()` - frees unmarked blocks in alloc
3. `dsweep()` - frees unmarked blocks in dalloc

## gc scratch space

to handle allocation failures (retry after gc), a fixed scratch space preserves values:

- 2 x i32 (pointers for `dconcat`, `dslice`)
- 1 x i64 (integer for `ditoa`)
- 1 x f64 (float for `dftoa`)

2 pointer slots is sufficient because nested expressions like `dconcat(dconcat(a, b), c)` evaluate inner-to-outer, so only 2 values need preservation at any call site.
