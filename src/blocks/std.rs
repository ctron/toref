use crate::runtime::fb::{DataInput, DataOutput, FunctionBlock};

pub struct Switch {
    g: DataInput,
}

impl Switch {
    pub fn new() -> Self {
        Self { g: DataInput {} }
    }
}

impl FunctionBlock for Switch {
    fn type_name(&self) -> String {
        "E_SR".to_string()
    }

    fn get_data_input(&self, name: &str) -> Option<DataInput> {
        match name {
            "G" => Some(self.g.clone()),
            _ => None,
        }
    }
}

pub struct Cycle {
    dt: DataInput,
}

impl Cycle {
    pub fn new() -> Self {
        Self { dt: DataInput {} }
    }
}

impl FunctionBlock for Cycle {
    fn type_name(&self) -> String {
        "E_CYCLE".to_string()
    }

    fn get_data_input(&self, name: &str) -> Option<DataInput> {
        match name {
            "DT" => Some(self.dt.clone()),
            _ => None,
        }
    }
}

pub struct SetReset {
    q: DataOutput,
}

impl SetReset {
    pub fn new() -> Self {
        Self { q: DataOutput {} }
    }
}

impl FunctionBlock for SetReset {
    fn type_name(&self) -> String {
        "E_SR".to_string()
    }

    fn get_data_output(&self, name: &str) -> Option<DataOutput> {
        match name {
            "Q" => Some(self.q.clone()),
            _ => None,
        }
    }
}
