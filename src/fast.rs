use crate::aast::AnalyzedStatement;

#[derive(Debug)]
pub struct FlattenedProgram {
    pub structs: Vec<AnalyzedStatement>,
    pub functions: Vec<AnalyzedStatement>,
}
