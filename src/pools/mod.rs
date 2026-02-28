use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub mod names;
pub mod codebase;
pub mod messages;
pub mod exprs;
pub mod items;

#[derive(Debug)]
pub struct PoolRef<T>(Arc<RwLock<T>>);

impl<T> PoolRef<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }
    pub fn lock(&self) -> RwLockReadGuard<T> {
        self.0.read().unwrap()
    }
    pub fn lock_mut(&self) -> RwLockWriteGuard<T> {
        self.0.write().unwrap()
    }
}

impl<T> Clone for PoolRef<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
