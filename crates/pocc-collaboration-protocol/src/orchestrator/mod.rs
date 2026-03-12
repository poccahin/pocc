use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SubTaskStatus { Pending, Completed }

#[derive(Debug, Clone)]
pub struct SubTaskNode {
    pub id: String,
    pub status: SubTaskStatus,
}

pub struct CompositeTask {
    pub sub_tasks: HashMap<String, SubTaskNode>,
}
