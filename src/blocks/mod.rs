use crate::runtime::factory::Creator;
use crate::runtime::fb::FunctionBlock;

pub mod std;

pub struct MockFunctionBlock(String);

impl MockFunctionBlock {
    pub fn creator(r#type: &str) -> impl Creator {
        let r#type = r#type.to_string();
        move || MockFunctionBlock(r#type.clone())
    }
}

impl FunctionBlock for MockFunctionBlock {
    fn type_name(&self) -> String {
        self.0.clone()
    }
}
