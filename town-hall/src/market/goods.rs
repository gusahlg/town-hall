use super::{Quantity, ResourceQuantity};
pub type GoodId = u16;
pub struct Good {
    pub configuration: Vec<ResourceQuantity>,
    pub id: GoodId,
}
pub type GoodQuantity = (Good, Quantity);
