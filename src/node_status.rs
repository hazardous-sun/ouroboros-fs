use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize)]
pub enum NodeStatus {
    Alive,
    Dead,
}
