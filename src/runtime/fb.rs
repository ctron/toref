use crate::protocol::server::{self, Data};
use crate::runtime::Request;

pub trait FunctionBlock: Send {
    fn type_name(&self) -> String;

    fn request(&mut self, request: Request) -> Result<Option<Data>, server::Error> {
        if request.destination.is_empty() {
            Err(server::Error::InvalidOperation)
        } else {
            Err(server::Error::InvalidDestination)
        }
    }

    fn get_data_output(&self, name: &str) -> Option<DataOutput> {
        None
    }

    fn get_data_input(&self, name: &str) -> Option<DataInput> {
        None
    }

    fn get_event_output(&self, name: &str) -> Option<EventOutput> {
        None
    }

    fn get_event_input(&self, name: &str) -> Option<EventInput> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct DataInput {}

#[derive(Clone, Debug)]
pub struct DataOutput {}

#[derive(Clone, Debug)]
pub struct EventInput {}

#[derive(Clone, Debug)]
pub struct EventOutput {}
