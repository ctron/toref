use crate::protocol::server::{Action, Data, Error};
use crate::runtime::container::SimpleContainer;
use crate::runtime::factory::FunctionBlockFactory;
use crate::runtime::fb::FunctionBlock;
use crate::runtime::Request;

pub struct EmbeddedResource<F>
where
    F: FunctionBlockFactory,
{
    container: SimpleContainer<F>,
}

impl<F> EmbeddedResource<F>
where
    F: FunctionBlockFactory,
{
    pub fn new(factory: F) -> Self {
        Self {
            container: SimpleContainer::new(factory),
        }
    }

    pub fn start(&mut self) {
        log::info!("Starting");
    }

    pub fn stop(&mut self) {
        log::info!("Stopping");
    }
}

impl<F> FunctionBlock for EmbeddedResource<F>
where
    F: FunctionBlockFactory + Send,
{
    fn type_name(&self) -> String {
        "EMB_RES".to_string()
    }

    fn request(&mut self, request: Request) -> Result<Option<Data>, Error> {
        log::info!("Request: {request:?}");

        if !request.destination.is_empty() {
            return Err(Error::InvalidDestination);
        }

        match (request.action, request.data) {
            (Action::Start, _) => {
                self.start();
                Ok(None)
            }
            (Action::Stop, _) => {
                self.stop();
                Ok(None)
            }
            (action, data) => self.container.process_request(Request {
                destination: Default::default(),
                action,
                data,
            }),
        }
    }
}
