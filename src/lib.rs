use dashmap::{DashMap, DashSet};
use rayon::{iter::Either, prelude::*};
use std::{
    hash::Hash,
    ops::Deref,
    rc::Rc,
    sync::{atomic::AtomicBool, Arc},
};
use thiserror::Error;
use tuples::TupleCloned;

#[cfg(test)]
mod tests;

pub trait DepMeta {
    type Id: Eq + Hash + Clone;

    fn get_id(&self) -> Self::Id;

    fn get_deps(&self) -> &[Self::Id];
}

mod impls {
    use crate::*;

    impl<T: DepMeta> DepMeta for &T {
        type Id = T::Id;

        fn get_id(&self) -> Self::Id {
            (**self).get_id()
        }

        fn get_deps(&self) -> &[Self::Id] {
            (**self).get_deps()
        }
    }

    impl<T: DepMeta> DepMeta for Rc<T> {
        type Id = T::Id;

        fn get_id(&self) -> Self::Id {
            self.deref().get_id()
        }

        fn get_deps(&self) -> &[Self::Id] {
            self.deref().get_deps()
        }
    }

    impl<T: DepMeta> DepMeta for Box<T> {
        type Id = T::Id;

        fn get_id(&self) -> Self::Id {
            self.deref().get_id()
        }

        fn get_deps(&self) -> &[Self::Id] {
            self.deref().get_deps()
        }
    }

    impl<T: DepMeta> DepMeta for Arc<T> {
        type Id = T::Id;

        fn get_id(&self) -> Self::Id {
            self.deref().get_id()
        }

        fn get_deps(&self) -> &[Self::Id] {
            self.deref().get_deps()
        }
    }
}

#[derive(Debug, Default)]
pub struct DepRes<Id: Eq + Hash + Clone> {
    ids: DashSet<Id>,
    deps: DashMap<Id, DashSet<Id>>,
}

impl<Id: Eq + Hash + Clone> DepRes<Id> {
    pub fn new() -> Self {
        Self {
            ids: DashSet::new(),
            deps: DashMap::new(),
        }
    }
}

impl<Id: Sync + Send + Eq + Hash + Clone> DepRes<Id> {
    pub fn add<'a>(
        &self,
        items: &'a impl IntoParallelRefIterator<'a, Item = impl DepMeta<Id = Id>>,
    ) {
        items.par_iter().for_each(|item| {
            let id = item.get_id();
            let deps = item.get_deps();
            let has_dep = !deps.is_empty();
            if has_dep {
                deps.par_iter().for_each(|dep| {
                    let dset = self
                        .deps
                        .entry(id.clone())
                        .or_insert_with(|| DashSet::new());
                    dset.insert(dep.clone());
                });
            }
            self.ids.insert(id);
        });
    }
}

#[derive(Debug, Default, Clone)]
pub struct ResolvedDeps<Id: Eq + Hash + Clone> {
    lvs: DashMap<usize, Arc<DashSet<Id>>>,
}

#[derive(Debug, Default, Clone)]
pub struct DepLevel<D> {
    pub level: usize,
    pub deps: D,
}

impl<Id: Eq + Hash + Clone> ResolvedDeps<Id> {
    fn new(lvs: DashMap<usize, Arc<DashSet<Id>>>) -> Self {
        Self { lvs }
    }

    pub fn sorted_by_level(&self) -> Vec<Id> {
        let mut vec = self.lvs.iter().collect::<Vec<_>>();
        vec.sort_by_key(|r| *r.key());
        let ids = vec
            .iter()
            .flat_map(|r| r.value().iter().map(|a| a.clone()))
            .collect::<Vec<_>>();
        ids
    }

    pub fn raw_level(&self) -> &DashMap<usize, Arc<DashSet<Id>>> {
        &self.lvs
    }

    pub fn iter_level(&self) -> impl Iterator<Item = DepLevel<Arc<DashSet<Id>>>> + '_ {
        self.lvs.iter().map(|kv| DepLevel {
            level: *kv.key(),
            deps: kv.value().clone(),
        })
    }
}

impl<Id: Sync + Send + Eq + Hash + Clone> DepRes<Id> {
    pub fn resolve(&mut self) -> Result<ResolvedDeps<Id>, DepResolveError> {
        let lvs = DashMap::new();

        if self.ids.is_empty() {
            return Ok(ResolvedDeps::new(lvs));
        }

        let (lv0, other): (DashSet<Id>, DashSet<Id>) = self.ids.par_iter().partition_map(|kv| {
            let id = kv.key().clone();
            if let None = self.deps.get(&id) {
                Either::Left(id.clone())
            } else {
                Either::Right(id.clone())
            }
        });
        if lv0.is_empty() {
            return Err(DepResolveError::IslandsOrCircular);
        }
        let lv0 = Arc::new(lv0);
        lvs.insert(0, lv0.cloned());

        let mut last = lv0;
        let mut other = other;
        let mut new_other = DashSet::new();
        let internal_data_error = AtomicBool::new(false);
        let mut lv = 1;
        loop {
            let lvn = DashSet::new();
            new_other.clear();

            other.par_iter().for_each(|id| {
                if let Some(deps) = self.deps.get(&*id) {
                    if deps.par_iter().any(|id| last.contains(&*id)) {
                        lvn.insert(id.clone());
                    } else {
                        new_other.insert(id.clone());
                    }
                } else {
                    internal_data_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            });

            if internal_data_error.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(DepResolveError::InternalDataError);
            }

            if lvn.is_empty() {
                if other.is_empty() {
                    return Ok(ResolvedDeps::new(lvs));
                } else {
                    return Err(DepResolveError::IslandsOrCircular);
                }
            }

            let lvn = Arc::new(lvn);
            lvs.insert(lv, lvn.cloned());
            last = lvn;
            std::mem::swap(&mut other, &mut new_other);
            lv += 1;
        }
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DepResolveError {
    #[error("There are islands or circular reference dependencies")]
    IslandsOrCircular,
    #[error("internal data error")]
    InternalDataError,
}
