use crate::aast::AnalyzedStatement;

#[derive(Debug)]
pub struct FlattenedProgram {
    pub structs: Vec<(AnalyzedStatement, u32, u32)>,
    pub functions: Vec<AnalyzedStatement>,
}
