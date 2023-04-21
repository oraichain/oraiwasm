pub trait CompositeKeyModel {
    fn get_composite_key(&self) -> String;
}

pub mod dataset;
pub mod offering;
