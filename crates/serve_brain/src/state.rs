use std::sync::Arc;

use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::messaging::Messaging;

#[derive(Clone)]
pub struct State {
    pub brains: Arc<Mutex<HashMap<String, Messaging>>>,
}

impl State {
    pub async fn try_get(&self, name: &str) -> Option<Messaging> {
        let brains = self.brains.lock().await;
        brains.get(name).map(Clone::clone)
    }
}
