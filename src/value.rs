

#[derive(Debug, Clone)]
pub enum Val {
    Int(u32),
    Signed(i32),
    Float(f32),
    BitStr(String),
    Vec(Vec<Val>),
    None,
    Error,
    // .. tbd
}
