use crate::blocks::std::{Cycle, SetReset, Switch};
use crate::runtime::fb::FunctionBlock;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CreationError {
    #[error("Unknown type")]
    UnknownType,
    #[error("Internal error")]
    Internal,
}

pub trait FunctionBlockFactory {
    fn create(&self, r#type: &str) -> Result<Box<dyn FunctionBlock>, CreationError>;
}

pub trait Creator: Send + Sync {
    fn create(&self) -> Box<dyn FunctionBlock>;
}

impl<F, T> Creator for F
where
    F: Fn() -> T + Send + Sync,
    T: FunctionBlock + 'static,
{
    fn create(&self) -> Box<dyn FunctionBlock> {
        Box::new((self)())
    }
}

#[derive(Clone)]
pub struct StandardFactory {
    types: Arc<RwLock<HashMap<String, Box<dyn Creator>>>>,
}

impl StandardFactory {
    pub fn new() -> Self {
        Self {
            types: Default::default(),
        }
    }

    pub fn register_type<N, C>(&mut self, name: N, creator: C)
    where
        N: Into<String>,
        C: Creator + 'static,
    {
        // FIXME: remove .unwrap()
        self.types
            .write()
            .unwrap()
            .insert(name.into(), Box::new(creator));
    }

    pub fn register_standard_types(&mut self) {
        self.register_type("E_SR", SetReset::new);
        self.register_type("E_CYCLE", Cycle::new);
        self.register_type("E_SWITCH", Switch::new);
    }
}

impl FunctionBlockFactory for StandardFactory {
    fn create(&self, r#type: &str) -> Result<Box<dyn FunctionBlock>, CreationError> {
        match self
            .types
            .read()
            .map_err(|_| CreationError::Internal)?
            .get(r#type)
        {
            Some(creator) => Ok(creator.create()),
            None => Err(CreationError::UnknownType),
        }
    }
}
