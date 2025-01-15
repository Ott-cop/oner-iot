use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: i32,
    pub name: String,
    pub state: bool
}

#[derive(Serialize, Deserialize)]
pub enum IO {
    PinInput,
    PinOutput
}