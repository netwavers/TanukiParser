use crate::ast::{EBNFInfo, Node, NodeKind, Token, TokenType, Span};
use crate::tokenizer::Tokenizer;
use typed_arena::Arena;

pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

pub struct EBNFParser<'a> {
    tokenizer: Tokenizer,
    arena: &'a Arena<Node<'a>>,
    token: Token,
    pub errors: Vec<Diagnostic>,
}

impl<'a> EBNFParser<'a> {
    pub fn new(tokenizer: Tokenizer, arena: &'a Arena<Node<'a>>) -> Self {
        let mut p = Self {
            tokenizer,
            arena,
            token: Token::default(),
            errors: Vec::new(),
        };
        p.token = p.tokenizer.get_token();
        println!("DEBUG: first token: {:?}, content: '{}'", p.token.token_type, p.token.content);
        p
    }

    fn push_error(&mut self, message: String, span: Span) {
        self.errors.push(Diagnostic { message, span });
    }

    fn peek(&self) -> &Token {
        &self.token
    }

    fn consume(&mut self) -> Token {
        let old = self.token.clone();
        self.token = self.tokenizer.get_token();
        old
    }

    // option = "[" alternation "]"
    fn parse_option(&mut self) -> Option<&'a Node<'a>> {
        let start_span = self.peek().span;
        if self.peek().token_type != TokenType::TOptionLeft {
            self.push_error(format!("Expected '[', found {:?}", self.peek().token_type), start_span);
            return None;
        }
        self.consume(); // [
        let node = self.parse_alternation()?;
        if self.peek().token_type != TokenType::TOptionRight {
            self.push_error(format!("Expected ']', found {:?}", self.peek().token_type), self.peek().span);
            return None;
        }
        let end_tok = self.consume(); // ]
        Some(self.arena.alloc(Node {
            kind: NodeKind::Option(node),
            span: Span::new(start_span.start, end_tok.span.end, start_span.line, start_span.column),
            embed_code: end_tok.embed_code,
        }))
    }

    // group = "(" alternation ")"
    fn parse_group(&mut self) -> Option<&'a Node<'a>> {
        let start_span = self.peek().span;
        if self.peek().token_type != TokenType::TGroupLeft {
            self.push_error(format!("Expected '(', found {:?}", self.peek().token_type), start_span);
            return None;
        }
        self.consume(); // (
        let node = self.parse_alternation()?;
        if self.peek().token_type != TokenType::TGroupRight {
            self.push_error(format!("Expected ')', found {:?}", self.peek().token_type), self.peek().span);
            return None;
        }
        let end_tok = self.consume(); // )
        Some(self.arena.alloc(Node {
            kind: NodeKind::Group(node),
            span: Span::new(start_span.start, end_tok.span.end, start_span.line, start_span.column),
            embed_code: end_tok.embed_code,
        }))
    }

    // element = rulename | group | option | char-val | num-val | prose-val
    fn parse_element(&mut self) -> Option<&'a Node<'a>> {
        let start_span = self.peek().span;
        let mut embed_code = String::new();
        let kind = match self.peek().token_type {
            TokenType::TRuleName => {
                let t = self.consume();
                embed_code = t.embed_code;
                NodeKind::RuleName {
                    name: t.content,
                    alias: 0,
                }
            }
            TokenType::TCharVal => {
                let t = self.consume();
                embed_code = t.embed_code;
                NodeKind::CharVal(t.content)
            }
            TokenType::TBinVal => NodeKind::BinVal(self.consume().content),
            TokenType::TDecVal => NodeKind::DecVal(self.consume().content),
            TokenType::THexVal => NodeKind::HexVal(self.consume().content),
            TokenType::TProseVal => NodeKind::ProseVal(self.consume().content),
            TokenType::TGroupLeft => return self.parse_group(),
            TokenType::TOptionLeft => return self.parse_option(),
            _ => {
                self.push_error(format!("Unexpected token in element: {:?}", self.peek().token_type), start_span);
                return None;
            }
        };
        let element_kind = self.arena.alloc(Node {
            kind,
            span: Span::new(start_span.start, start_span.end, start_span.line, start_span.column), // Simplified end
            embed_code,
        });
        Some(self.arena.alloc(Node {
            kind: NodeKind::Element(element_kind),
            span: element_kind.span,
            embed_code: String::new(),
        }))
    }

    fn parse_repeat(&mut self) -> Option<&'a Node<'a>> {
        let start_span = self.peek().span;
        let kind = match self.peek().token_type {
            TokenType::TAsterisk => {
                self.consume();
                NodeKind::Repeat0
            }
            TokenType::TPlus => {
                self.consume();
                NodeKind::Repeat1
            }
            _ => {
                self.push_error(format!("Expected '*' or '+', found {:?}", self.peek().token_type), start_span);
                return None;
            }
        };
        Some(self.arena.alloc(Node {
            kind,
            span: start_span,
            embed_code: String::new(),
        }))
    }

    // repetition = element [ repeat ]
    fn parse_repetition(&mut self) -> Option<&'a Node<'a>> {
        let element = self.parse_element()?;
        let start_span = element.span;
        let mut repeat = None;
        let mut end_span = start_span;
        if self.peek().token_type == TokenType::TAsterisk || self.peek().token_type == TokenType::TPlus {
            let r = self.parse_repeat()?;
            end_span = r.span;
            repeat = Some(r);
        }
        Some(self.arena.alloc(Node {
            kind: NodeKind::Repetition { element, repeat },
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
            embed_code: String::new(),
        }))
    }

    // concatenation = repetition ( repetition )*
    fn parse_concatenation(&mut self) -> Option<&'a Node<'a>> {
        let first = self.parse_repetition()?;
        let start_span = first.span;
        let mut list = vec![first];
        let mut end_span = start_span;
        
        while matches!(self.peek().token_type, 
            TokenType::TRuleName | TokenType::TCharVal | TokenType::TBinVal |
            TokenType::TDecVal | TokenType::THexVal | TokenType::TProseVal |
            TokenType::TGroupLeft | TokenType::TOptionLeft) 
        {
            let rep = self.parse_repetition()?;
            end_span = rep.span;
            list.push(rep);
        }
        
        Some(self.arena.alloc(Node {
            kind: NodeKind::Concatenation(list),
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
            embed_code: String::new(),
        }))
    }

    // alternation = concatenation ( "|" concatenation )*
    fn parse_alternation(&mut self) -> Option<&'a Node<'a>> {
        let first = self.parse_concatenation()?;
        let start_span = first.span;
        let mut list = vec![first];
        let mut end_span = start_span;
        while self.peek().token_type == TokenType::TAlternative {
            self.consume(); // |
            let conc = self.parse_concatenation()?;
            end_span = conc.span;
            list.push(conc);
        }
        Some(self.arena.alloc(Node {
            kind: NodeKind::Alternation(list),
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
            embed_code: String::new(),
        }))
    }

    fn parse_elements(&mut self) -> Option<&'a Node<'a>> {
        let node = self.parse_alternation()?;
        Some(self.arena.alloc(Node {
            kind: NodeKind::Elements(node),
            span: node.span,
            embed_code: String::new(),
        }))
    }

    // rule = rulename "=" elements
    fn parse_rule(&mut self) -> Option<&'a Node<'a>> {
        let start_span = self.peek().span;
        if self.peek().token_type != TokenType::TRuleName {
            self.push_error(format!("Expected rule name, found {:?}", self.peek().token_type), start_span);
            return None;
        }
        let name_tok = self.consume();
        if self.peek().token_type != TokenType::TEqual {
            self.push_error(format!("Expected '=', found {:?}", self.peek().token_type), self.peek().span);
            return None;
        }
        self.consume(); // =
        let elements = self.parse_elements()?;
        let mut end_span = elements.span;
        if self.peek().token_type == TokenType::TNl {
            end_span = self.consume().span;
        } else if self.peek().token_type != TokenType::TEof {
            self.push_error(format!("Expected newline or EOF, found {:?}", self.peek().token_type), self.peek().span);
        }

        Some(self.arena.alloc(Node {
            kind: NodeKind::Rule {
                name: name_tok.content,
                elements,
            },
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
            embed_code: String::new(),
        }))
    }

    fn parse_rulelist(&mut self) -> Option<&'a Node<'a>> {
        let start_span = self.peek().span;
        
        let mut rules = Vec::new();
        let mut end_span = start_span;

        while self.peek().token_type != TokenType::TEof {
            let tok_type = self.peek().token_type;
            if tok_type == TokenType::TNl || tok_type == TokenType::TBody {
                self.consume();
                continue;
            }
            
            if tok_type == TokenType::TRuleName {
                if let Some(rule) = self.parse_rule() {
                    end_span = rule.span;
                    rules.push(rule);
                } else {
                    // Error already pushed in parse_rule.
                    // Recovery: Skip until next NL or EOF to try next rule
                    while self.peek().token_type != TokenType::TNl && self.peek().token_type != TokenType::TEof {
                        self.consume();
                    }
                }
            } else {
                // Unexpected token where a rule name was expected
                self.push_error(format!("Unexpected token outside rule: {:?}", tok_type), self.peek().span);
                self.consume();
            }
        }

        if rules.is_empty() && self.errors.is_empty() {
            self.push_error("No rules found in EBNF definition".into(), start_span);
            return None;
        }

        Some(self.arena.alloc(Node {
            kind: NodeKind::RuleList(rules),
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
            embed_code: String::new(),
        }))
    }

    fn parse_namespace(&mut self) -> Option<String> {
        if self.peek().token_type != TokenType::TNamespace {
            self.push_error(format!("Expected '@namespace', found {:?}", self.peek().token_type), self.peek().span);
            return None;
        }
        self.consume();
        if self.peek().token_type != TokenType::TRuleName {
            self.push_error(format!("Expected namespace name, found {:?}", self.peek().token_type), self.peek().span);
            return None;
        }
        let name = self.consume().content;
        while self.peek().token_type == TokenType::TNl {
            self.consume();
        }
        Some(name)
    }

    fn parse_class_name(&mut self) -> Option<String> {
        if self.peek().token_type != TokenType::TClass {
            self.push_error(format!("Expected '@class', found {:?}", self.peek().token_type), self.peek().span);
            return None;
        }
        self.consume();
        if self.peek().token_type != TokenType::TRuleName {
            self.push_error(format!("Expected class name, found {:?}", self.peek().token_type), self.peek().span);
            return None;
        }
        let name = self.consume().content;
        while self.peek().token_type == TokenType::TNl {
            self.consume();
        }
        Some(name)
    }

    fn parse_token_list(&mut self) -> Vec<&'a Node<'a>> {
        let mut tokens: Vec<&'a Node<'a>> = Vec::new();
        while self.peek().token_type == TokenType::TToken {
            let start_span = self.peek().span;
            self.consume();
            if self.peek().token_type != TokenType::TRuleName {
                break;
            }
            let name = self.consume().content;
            let mut char_val = None;
            let mut end_span = self.token.span;
            if self.peek().token_type == TokenType::TCharVal {
                let t = self.consume();
                char_val = Some(t.content);
                end_span = t.span;
            }
            tokens.push(self.arena.alloc(Node {
                kind: NodeKind::TokenDef { name, char_val },
                span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                embed_code: String::new(),
            }));
            
            while self.peek().token_type == TokenType::TNl {
                self.consume();
            }
        }
        tokens
    }

    fn parse_user_define(&mut self) -> Option<Vec<String>> {
        if self.peek().token_type != TokenType::TUserDefineLeft {
            return None;
        }
        // The tokenizer is currently after '@{'. get_lines_until_end_block will read until '@}'.
        let lines = self.tokenizer.get_lines_until_end_block();
        
        // After reading lines, we must sync the current token.
        // The tokenizer position is now at '@'.
        self.token = self.tokenizer.get_token(); // This should get TUserDefineRight
        
        if self.token.token_type == TokenType::TUserDefineRight {
            self.token = self.tokenizer.get_token(); // This gets the first token after @}
        }
        
        // skip NLs
        while self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        Some(lines)
    }

    fn parse_decl_list(&mut self) -> Vec<&'a Node<'a>> {
        let mut decls: Vec<&'a Node<'a>> = Vec::new();
        while self.peek().token_type == TokenType::TDecl {
            let start_span = self.peek().span;
            self.consume();
            if self.peek().token_type != TokenType::TRuleName { break; }
            let d_type = self.consume().content;
            if self.peek().token_type != TokenType::TRuleName { break; }
            let d_name = self.consume().content;
            let end_span = self.token.span;

            decls.push(self.arena.alloc(Node {
                kind: NodeKind::Decl {
                    decl_type: d_type,
                    name: d_name,
                    init: None,
                },
                span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                embed_code: String::new(),
            }));
            while self.peek().token_type == TokenType::TNl {
                self.consume();
            }
        }
        decls
    }

    pub fn parse(&mut self) -> Option<EBNFInfo<'a>> {
        let user_define = self.parse_user_define();
        let namespace = self.parse_namespace()?;
        let class_name = self.parse_class_name()?;
        let decl_list = self.parse_decl_list();
        let token_list = self.parse_token_list();
        let rule_list_node = self.parse_rulelist()?;
        
        let rules = if let NodeKind::RuleList(r) = &rule_list_node.kind {
            r.clone()
        } else {
            vec![]
        };

        Some(EBNFInfo {
            user_define,
            namespace,
            class_name,
            decls: decl_list,
            tokens: token_list,
            rules,
        })
    }
}
