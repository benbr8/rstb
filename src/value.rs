

#[derive(Debug, Clone)]
pub enum RstbValue {
    Int(i32),
    Float(f32),
    BitStr(String),
    None
    // .. tbd
}