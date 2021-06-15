

#[derive(Debug, Clone)]
pub enum RstbValue {
    Int(i32),
    Float(f32),
    BitStr(String),
    Vec(Vec<RstbValue>),
    None
    // .. tbd
}