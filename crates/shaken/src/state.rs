use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use anyhow::Context as _;
use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};

#[derive(Default, Clone)]
pub struct SharedState(Arc<RwLock<State>>);

impl SharedState {
    pub fn new(state: State) -> Self {
        Self(Arc::new(RwLock::new(state)))
    }

    pub async fn get<T>(&self) -> RwLockReadGuard<'_, T>
    where
        T: Any + Send + Sync + 'static,
    {
        RwLockReadGuard::map(self.0.read().await, |state| state.get::<T>().unwrap())
    }

    pub async fn try_get<T>(&self) -> Option<RwLockReadGuard<'_, T>>
    where
        T: Any + Send + Sync + 'static,
    {
        RwLockReadGuard::try_map(self.0.read().await, |state| state.get::<T>().ok()).ok()
    }

    pub async fn get_mut<T>(&self) -> RwLockMappedWriteGuard<'_, T>
    where
        T: Any + Send + Sync + 'static,
    {
        RwLockWriteGuard::map(self.0.write().await, |state| state.get_mut::<T>().unwrap())
    }

    pub async fn try_get_mut<T>(&self) -> Option<RwLockMappedWriteGuard<'_, T>>
    where
        T: Any + Send + Sync + 'static,
    {
        RwLockWriteGuard::try_map(self.0.write().await, |state| state.get_mut::<T>().ok()).ok()
    }

    pub async fn insert<T>(&self, val: T)
    where
        T: Any + Send + Sync + 'static,
    {
        self.0.write().await.insert(val);
    }

    pub async fn extract<T, U, F>(&self, map: F) -> RwLockReadGuard<'_, U>
    where
        T: Any + Send + Sync + 'static,
        U: Send + 'static,
        F: FnOnce(&T) -> &U + Send,
    {
        RwLockReadGuard::map(self.get::<T>().await, map)
    }
}

#[derive(Default, Debug)]
pub struct State {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl State {
    pub fn insert<T>(&mut self, val: T)
    where
        T: Any + Send + Sync + 'static,
    {
        if let Some(..) = self.map.insert(TypeId::of::<T>(), Box::new(val)) {
            eprintln!("override: {}", std::any::type_name::<T>());
        }
    }

    pub fn with<T>(mut self, val: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        self.insert(val);
        self
    }

    pub fn extract<'a, T, U, F>(&'a self, map: F) -> anyhow::Result<&'a U>
    where
        T: Any + Send + Sync + 'static,
        U: Send + 'static,
        F: Fn(&'a T) -> &'a U,
    {
        self.get::<T>().map(map)
    }

    pub fn get<T>(&self) -> anyhow::Result<&T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|c| c.downcast_ref())
            .with_context(|| anyhow::anyhow!("could not find {}", Self::name_of::<T>()))
    }

    pub fn get_mut<T>(&mut self) -> anyhow::Result<&mut T>
    where
        T: Any + Send + Sync + 'static,
    {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|c| c.downcast_mut())
            .with_context(|| anyhow::anyhow!("could not find {}", Self::name_of::<T>()))
    }

    pub fn has<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        self.get::<T>().is_ok()
    }

    fn name_of<T: 'static>() -> &'static str {
        std::any::type_name::<T>()
    }
}
