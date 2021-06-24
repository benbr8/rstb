

#[derive(Debug, Clone)]
pub enum Val {
    Int(i32),
    Float(f32),
    BitStr(String),
    Vec(Vec<Val>),
    None,
    Error,
    // .. tbd
}
