# Lua's Design

A simple description of lua's design

## Description

- garbage collected
- uses bytecode. 
- interpreted/jit 
- written in ansi c-
- very portable-
- tiny binary + stdlib
- Does not allow for inheritence.

### Possible changes

- add an `impl` & `like` system
  - a DS will have a `preimpl` and `prelike` and also a `impl` and `like`
  - `impl`: a set of methods, `like`: a subset `impl`, similar to traits
- something like `comptime` instead of `impl-like`, call it `comp`
  - all `predef` values can be  `comp`.
  - only `predef` values can be used in `comp`
- tagged blocks

## Data Structures

- `nil`: the type of a single value called nil;
- `number`: floating point numbers;
- `string`: arrays of characters;
- `function`: user defined functions;
- `Cfunction`: functions provided by the host program;
- `userdata`: pointers to host data;
- `table`: associative arrays. 

### Possible changes

- no `nil` type. nullability differs by type, some not allowing it.
- more complex number system:
  - `signed` type
  - `unsigned` type 
  - `decimal` type
    - explicit conversions between them
- multiple string types:
  - `astr` ascii string
  - `bstr` non-descript byte string
  - `ustr` utf8 string
- `function` keep as same, maybe rename to `fn`
- use `prefn` and `prelet`, `preconst`, `predef`
- maybe no `userdata` type?
- `table` probably keep the same, will need to see in the future.
- could maybe also create a `box` type
- struct like ds
- enum like ds

## other details

Lua is single threaded.

### Possible changes

Add some kind of multi-threading support? Maybe a `libfn`
