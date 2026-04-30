#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tag {
    pub number: u32,
    pub active: bool,
    pub urgent: bool,
    pub occupied: bool,
    pub focused_client: bool,
}
