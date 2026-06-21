# animagram-rule

Here are common rules for projects in animagram.

## Contribution

### Community

#### Issue

- tags

| Label | Description | Color |
|-|-|-|
| bug | Something isn't working. | #d73a4a |
| controverse | What we should talk about. | #5B2F91 |
| improvement | A way we can improve the repository. | #259d63 |

### Writing

This is for easy reading by human and computers.

- Use tab(0x09) or 4 spaces(0x20) for indentation.

#### Directory and file

- READEME.md: project name, problem & solution, quick start, commands, etc.
- docs/Architecture.md: structure of requirements, functions, diagram of processes, modules, script files, etc.
- LICENSE: apache-2.0, author.
- .gitignore: *.lock, target, etc.
- src: script files.
- examples: static data files, datasets for test, example, etc.

#### Name format

**Single word naming is always best.**

- directory:  snake_case (Kebab-case is also good unless for script.)
- file:
    - Document: CamelCase
    - script:   snake_case
        - varaiable: snake_case (with kebab-case is also good in static format.)
    - data_format: snake_case
        - key:  snake_case (with kebab-case for 2 diffrent degrees of relation.)
- md outline: Capitalized with space
- sentence:   Capitalized with space and .

Abbreviations follow the same rule as others.

### Document

This is for easy reading and comprehensive understanding of all within a repository by human.

When written in Japanese for maintainability, state on the 1st line the following:

```text
// This file includes untranslated text (ja).
```

### Script

Write scripts with only ASCII.
The following example uses Rust. When using a different stack, adapt it accordingly.
These are for comprehensive understanding of scripts.

#### Dependencies

- Dependencies refer not only to third-party modules but to all callings defined outside of itself.
- Declare it not only in root (lib.rs), but each script file in its 1st block.
- Prohibit individual inline references below the declaration block.
- Order: `core` → `alloc` → `std` → `crate` → those with attributes (`core` → `alloc` → `std` → `crate`).
- Witout projects heap limitations are never useful, include a commented-out `no_std` declaration in the root file in advance.
- The ideal approach is a single feature that works in both `std` or `no_std` environments.

```rust
// examples

// #![no_std]
extern crate core;
extern crate alloc;
extern crate std;
use core::{
    primitive::{u8, u64, usize, i64, str},
    fmt::{Display, Result, Formatter},
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

#[cfg(test)]
use std::fs;

use crate::{
    list::{List, VariableList},
    debug_log,
}
```

#### Error

This is for comprehensive export of error within softwear for me, and comprehensive implement for users.

- Do not use `std::error::Error` implementations as they are not compatible with no_std.
- Define an item for each module (such as `ListError`), and wrap them in the public Error item (e.g., `VariableListError::List(ListError)`).

```rust
// examples

#[derive(Debug)]
pub enum ListError {
    OutOfBounds,
    NotExist,
}
#[derive(Debug)]
pub enum VariableListError {
    List(ListError),
    Compact,
}

impl Display for ListError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:?}", self)
    }
}
```

#### Literal quote

This is for easy of stack changing.

- slicable string: "" e.g., "", "ab"
- unslicable char: '' e.g., 'あ'

#### Comment

This is for minimum maintenance costs.

- Write a outer line DocComment for each public item.
- Write a DocTest for each public fun. Skip meaningless tests(e.g., Item:new) and comments.
- Write a DocTest when needed for each private fun.
- Inside functions, clearly state the intent with inline comments where necessary.

#### Test

This is for minimum maintenance costs.

- Write unit tests that is not duplicating with DocTest.
- Tests can depend on std::fs and the examples directory. Avoid inline dataset definitions.
- Names of test functions should follow the format `{target}_{condition}` (omit test_).
- When integration test, using the examples directory, in-memory mock implementations to verify exported functions.

#### Html

- formatting rule:
    - Do not insert a line break before a closing tag.
    - Insert a line break before the start of every tag.
