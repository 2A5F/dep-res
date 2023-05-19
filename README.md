# dep-res

![Rust](https://github.com/2A5F/dep-res/workflows/Rust/badge.svg)
[![version](https://img.shields.io/crates/v/dep-res)](https://crates.io/crates/dep-res)
[![documentation](https://docs.rs/dep-res/badge.svg)](https://docs.rs/dep-res)
![LICENSE](https://img.shields.io/crates/l/dep-res)

Simple dependency resolution

## Example

```rust
#[derive(Debug)]
struct SimpleDep {
    pub id: usize,
    pub deps: Vec<usize>,
}

impl DepMeta for SimpleDep {
    type Id = usize;

    fn get_id(&self) -> Self::Id {
        self.id
    }

    fn get_deps(&self) -> &[Self::Id] {
        &*self.deps
    }
}

let items = vec![
    SimpleDep {
        id: 0,
        deps: vec![],
    },
    SimpleDep {
        id: 1,
        deps: vec![0],
    },
    SimpleDep {
        id: 2,
        deps: vec![],
    },
    SimpleDep {
        id: 3,
        deps: vec![],
    },
    SimpleDep {
        id: 4,
        deps: vec![3],
    },
    SimpleDep {
        id: 5,
        deps: vec![4],
    },
];

let mut dr = DepRes::new();
dr.add(&items);
let r = dr.resolve().unwrap();

let items = r.sorted_by_level();
println!("{:?}", items);
// [0, 2, 3, 1, 4, 5]

let levels = r.iter_level().collect::<Vec<_>>();
println!("{:?}", levels);
// // example, actually unordered
//
// [ 
//    DepLevel { level: 0, deps: {0: (), 2: (), 3: ()} },
//    DepLevel { level: 1, deps: {1: (), 4: ()} }, 
//    DepLevel { level: 2, deps: {5: ()} }, 
// ]
```
