use crate::*;

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

#[test]
fn test_1() {
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
    let r = dr.resolve();
    println!("{:?}\n", r);
    println!("{:?}\n", dr);
    let r = r.unwrap();
    let items = r.sorted_by_level();
    println!("{:?}", items);
    println!("");
    let levels = r.iter_level().collect::<Vec<_>>();
    println!("{:?}", levels);
}
