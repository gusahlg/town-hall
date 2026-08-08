use super::GoodId;
pub type Quantity = u16;

pub struct RecipeInput {
    pub good: GoodId,
    pub quantity: Quantity,
}
pub struct Recipe {
    pub inputs: Vec<RecipeInput>,
    pub processes: Vec<Process>,
}

pub struct Process {
    pub workers: u16,
    pub duration: u16,
    pub equipment: Vec<(GoodId, Quantity)>,
    // IMplement this later
    // inefficiency: Vec<GoodId>,
}
