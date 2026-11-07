pub mod handler;
pub mod import;
pub mod investigator;
pub mod page;
pub mod roll;

crate::enumerate! {
    pub Kind(u8) {
        Other,
        Investigator,
    }
}
