use crate::protocol::server;
use crate::protocol::server::{Action, Data, Error};
use crate::runtime::factory::FunctionBlockFactory;
use crate::runtime::fb::FunctionBlock;
use crate::runtime::Request;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, thiserror::Error)]
pub enum AddError {
    #[error("Item with that name already exists")]
    ItemAlreadyExist,
    #[error("Unknown type")]
    UnknownType,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Unknown block")]
    UnknownBlock,
    #[error("Unknown port")]
    UnknownPort,
}

pub trait Container {
    /// Add a child to the container
    fn add_child(&mut self, name: String, r#ype: &str) -> Result<(), AddError>;

    /// Remove a child from the container
    fn remove_child(&mut self, name: &str);

    /// Connect two ports
    fn connect(
        &mut self,
        source: PortDestination,
        destination: PortDestination,
    ) -> Result<(), ConnectError>;

    /// Disconnect two ports
    fn disconnect(
        &mut self,
        source: PortDestination,
        destination: PortDestination,
    ) -> Result<(), ConnectError>;
}

pub struct SimpleContainer<F>
where
    F: FunctionBlockFactory,
{
    factory: F,
    children: HashMap<String, Box<dyn FunctionBlock>>,
}

impl<F> SimpleContainer<F>
where
    F: FunctionBlockFactory,
{
    pub fn process_request(&mut self, request: Request) -> Result<Option<Data>, Error> {
        log::debug!("Processing request: {request:?}");

        match request.pop() {
            (Some(next), request) => self.child_process_request(next, request),
            (None, request) => self.local_process_request(request),
        }
    }

    fn child_process_request(
        &mut self,
        next: String,
        request: Request,
    ) -> Result<Option<Data>, Error> {
        match self.children.get_mut(&next) {
            Some(child) => child.request(request),
            None => Err(Error::InvalidDestination),
        }
    }

    fn local_process_request(&mut self, request: Request) -> Result<Option<Data>, Error> {
        Ok(match (request.action, request.data) {
            (Action::Query, Some(Data::FunctionBlock { name, r#type })) => {
                if name == "*" && r#type == "*" {
                    if self.children.is_empty() {
                        None
                    } else {
                        Some(Data::FunctionBlockList(
                            self.children
                                .iter()
                                .map(|(name, fb)| server::FunctionBlock {
                                    name: name.to_string(),
                                    r#type: fb.type_name(),
                                })
                                .collect(),
                        ))
                    }
                } else {
                    None
                }
            }
            (Action::Create, Some(Data::FunctionBlock { name, r#type })) => {
                self.add_child(name, &r#type).map(|_| None)?
            }
            (Action::Delete, Some(Data::FunctionBlock { name, .. })) => {
                self.remove_child(&name);
                None
            }
            (
                Action::Create,
                Some(Data::Connection {
                    source,
                    destination,
                }),
            ) => self
                .connect(
                    source.try_into().map_err(|()| Error::InvalidDestination)?,
                    destination
                        .try_into()
                        .map_err(|()| Error::InvalidDestination)?,
                )
                .map(|_| None)?,
            (
                Action::Delete,
                Some(Data::Connection {
                    source,
                    destination,
                }),
            ) => self
                .disconnect(
                    source.try_into().map_err(|()| Error::InvalidDestination)?,
                    destination
                        .try_into()
                        .map_err(|()| Error::InvalidDestination)?,
                )
                .map(|_| None)?,
            (Action::Read, Some(Data::Watches)) => None,
            _ => return Err(Error::InvalidOperation),
        })
    }
}

impl<F> SimpleContainer<F>
where
    F: FunctionBlockFactory,
{
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            children: HashMap::new(),
        }
    }
}

impl<F> Container for SimpleContainer<F>
where
    F: FunctionBlockFactory,
{
    fn add_child(&mut self, name: String, r#type: &str) -> Result<(), AddError> {
        log::info!("Adding {name} of type {type}");

        match self.children.entry(name) {
            Entry::Vacant(entry) => match self.factory.create(r#type) {
                Ok(child) => {
                    entry.insert(child);
                    log::info!("Item added");
                    Ok(())
                }
                Err(_) => {
                    log::warn!("Unknown type '{type}'");
                    Err(AddError::UnknownType)
                }
            },
            Entry::Occupied(_) => {
                log::warn!("Item already exists");
                Err(AddError::ItemAlreadyExist)
            }
        }
    }

    fn remove_child(&mut self, name: &str) {
        log::info!("Removing: {name}");
        self.children.remove(name);
    }

    fn connect(
        &mut self,
        source: PortDestination,
        destination: PortDestination,
    ) -> Result<(), ConnectError> {
        log::info!("Connect: {source} -> {destination}");

        let source_fb = self
            .children
            .get_mut(&source.block)
            .ok_or(ConnectError::UnknownBlock)?;

        let source_port = source_fb
            .get_data_output(&source.port)
            .ok_or(ConnectError::UnknownPort)?;

        let destination_fb = self
            .children
            .get_mut(&destination.block)
            .ok_or(ConnectError::UnknownBlock)?;

        let destination_port = destination_fb
            .get_data_input(&destination.port)
            .ok_or(ConnectError::UnknownPort)?;

        log::info!("Creating new connection");

        // FIXME: implement actual binding

        Ok(())
    }

    fn disconnect(
        &mut self,
        source: PortDestination,
        destination: PortDestination,
    ) -> Result<(), ConnectError> {
        log::info!("Disconnect: {source} -> {destination}");
        todo!();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PortDestination {
    block: String,
    port: String,
}

impl Display for PortDestination {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}.{1}", self.block, self.port)
    }
}

impl PortDestination {
    pub fn from_str(name: &str) -> Result<Self, ()> {
        let s = name.split('.').collect::<Vec<_>>();
        if s.len() != 2 {
            Err(())
        } else {
            Ok(Self {
                block: s[0].to_string(),
                port: s[1].to_string(),
            })
        }
    }
}

impl TryFrom<String> for PortDestination {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        PortDestination::from_str(&value)
    }
}
