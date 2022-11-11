use crate::runtime::emb_res::EmbeddedResource;
use crate::runtime::factory::{CreationError, FunctionBlockFactory};
use crate::runtime::fb::FunctionBlock;

pub struct RootFactory<F>
where
    F: FunctionBlockFactory,
{
    factory: F,
}

impl<F> RootFactory<F>
where
    F: FunctionBlockFactory,
{
    pub fn new(factory: F) -> Self {
        Self { factory }
    }
}

impl<F> FunctionBlockFactory for RootFactory<F>
where
    F: FunctionBlockFactory + Send + Clone + 'static,
{
    fn create(&self, r#type: &str) -> Result<Box<dyn FunctionBlock>, CreationError> {
        match r#type {
            "EMB_RES" => Ok(Box::new(EmbeddedResource::new(self.factory.clone()))),
            _ => Err(CreationError::UnknownType),
        }
    }
}
