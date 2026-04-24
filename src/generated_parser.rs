use crate::ast::{Node, TokenType};

#[allow(dead_code, unused_mut, unused_variables, unused_assignments, non_snake_case)]
pub struct Test<'a> {
    pub tokenizer: &'a mut crate::tokenizer::Tokenizer,
    pub token: crate::ast::Token,
}

impl<'a> Test<'a> {
    fn rule(&mut self) -> Option<Node<'a>> {
        None
    }
    
    pub fn parse(&mut self) -> Option<Node<'a>> {
        self.rule()
    }
}
