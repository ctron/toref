pub mod container;
pub mod emb_res;
pub mod factory;
pub mod fb;
pub mod root;

use crate::protocol::server::{Action, Data, Error};
use crate::protocol::RequestTarget;
use crate::runtime::container::SimpleContainer;
use crate::runtime::factory::StandardFactory;
use crate::runtime::root::RootFactory;
use async_trait::async_trait;
use std::ops::{Deref, DerefMut};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeError {}

pub struct Runtime {
    root: SimpleContainer<RootFactory<StandardFactory>>,
    rx: Receiver<RequestHandle>,
    tx: Sender<RequestHandle>,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub destination: Destination,
    pub action: Action,
    pub data: Option<Data>,
}

impl Request {
    /// Pop one element from the destination, return the result and self.
    ///
    /// If the destination was option, the first element of the tuple will be [`None`].
    pub fn pop(mut self) -> (Option<String>, Self) {
        (self.destination.pop(), self)
    }
}

struct RequestHandle {
    pub request: Request,
    pub tx: oneshot::Sender<Result<Option<Data>, Error>>,
}

#[derive(Clone, Debug)]
pub struct Requests {
    tx: Sender<RequestHandle>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Destination(Vec<String>);

impl Destination {
    pub fn from_str(destination: &str) -> Destination {
        if destination.is_empty() {
            return Destination(vec![]);
        }
        Self(destination.split('.').map(|s| s.to_string()).collect())
    }
}

impl Deref for Destination {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Destination {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Requests {
    pub async fn request(
        &self,
        destination: String,
        action: Action,
        data: Option<Data>,
    ) -> Result<Option<Data>, Error> {
        let (tx, rx) = oneshot::channel();

        let destination = Destination::from_str(&destination);

        let request = RequestHandle {
            request: Request {
                destination,
                action,
                data,
            },
            tx,
        };

        if let Err(_) = self.tx.send(request).await {
            return Err(Error::NotReady);
        }

        match rx.await {
            Ok(r) => r,
            Err(_) => Err(Error::NotReady),
        }
    }
}

#[async_trait]
impl RequestTarget for Requests {
    async fn process_request(
        &self,
        destination: String,
        action: Action,
        data: Option<Data>,
    ) -> Result<Option<Data>, Error> {
        self.request(destination, action, data).await
    }
}

impl Runtime {
    pub fn new(factory: StandardFactory) -> Self {
        let (tx, rx) = mpsc::channel(128);

        Self {
            root: SimpleContainer::new(RootFactory::new(factory)),
            rx,
            tx,
        }
    }

    /// Create a new Requests sender.
    pub fn requests(&self) -> Requests {
        Requests {
            tx: self.tx.clone(),
        }
    }

    pub async fn run(mut self) -> Result<(), RuntimeError> {
        loop {
            match self.rx.recv().await {
                None => return Ok(()),
                Some(msg) => {
                    let _ = msg.tx.send(self.root.process_request(msg.request));
                }
            }
        }
    }
}
