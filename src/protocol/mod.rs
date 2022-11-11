use crate::protocol::server::{Action, Data, Error};
use async_trait::async_trait;

pub mod server;

/// A trait which can handle requests from the protocol server.
#[async_trait]
pub trait RequestTarget: Clone + Send + Sync {
    async fn process_request(
        &self,
        destination: String,
        action: Action,
        data: Option<Data>,
    ) -> Result<Option<Data>, Error>;
}
