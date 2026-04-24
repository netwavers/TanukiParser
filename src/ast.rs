// use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self { start, end, line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    TRuleName,
    TCharVal,
    TBinVal,
    TDecVal,
    THexVal,
    TProseVal,
    TEqual,
    TAlternative,
    TPlus,
    TAsterisk,
    TGroupLeft,
    TGroupRight,
    TOptionLeft,
    TOptionRight,
    TAmpersand,
    TPiriod,
    TNl,
    TEof,
    TBody,           // @@
    TUserDefineLeft,  // @{
    TUserDefineRight, // @}
    TNamespace,      // @namespace
    TClass,          // @class
    TDecl,           // @decl
    TToken,          // @token
    TError,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub content: String,
    pub span: Span,
    pub embed_code: String,
}

impl Default for Token {
    fn default() -> Self {
        Self {
            token_type: TokenType::TError,
            content: String::new(),
            span: Span::default(),
            embed_code: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub kind: NodeKind<'a>,
    pub span: Span,
    pub embed_code: String,
}

#[derive(Debug, Clone)]
pub enum NodeKind<'a> {
    RuleList(Vec<&'a Node<'a>>),
    Rule {
        name: String,
        elements: &'a Node<'a>,
    },
    Elements(&'a Node<'a>),
    Alternation(Vec<&'a Node<'a>>),
    Concatenation(Vec<&'a Node<'a>>),
    Repetition {
        element: &'a Node<'a>,
        repeat: Option<&'a Node<'a>>,
    },
    Element(&'a Node<'a>),
    Group(&'a Node<'a>),
    Option(&'a Node<'a>),
    RuleName {
        name: String,
        alias: i32,
    },
    CharVal(String),
    BinVal(String),
    DecVal(String),
    HexVal(String),
    ProseVal(String),
    Repeat0,
    Repeat1,
    Namespace(String),
    Class(String),
    TokenDef {
        name: String,
        char_val: Option<String>,
    },
    UserDefine(Vec<String>),
    Decl {
        decl_type: String,
        name: String,
        init: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct NodeData<'a> {
    pub node: &'a Node<'a>,
    pub embed_code: String,
}

#[derive(Debug, Clone)]
pub struct EBNFInfo<'a> {
    pub user_define: Option<Vec<String>>,
    pub namespace: String,
    pub class_name: String,
    pub decls: Vec<&'a Node<'a>>,
    pub tokens: Vec<&'a Node<'a>>,
    pub rules: Vec<&'a Node<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetLanguage {
    CSharp,
    Rust,
    Python,
}

impl Default for TargetLanguage {
    fn default() -> Self {
        TargetLanguage::CSharp
    }
}

// Tree structure for Code Generation
#[derive(Debug, Clone)]
pub struct Tree<'a> {
    pub kind: TreeKind<'a>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TreeKind<'a> {
    Namespace {
        name: String,
        tokens: Vec<String>,
        class_body: &'a Tree<'a>,
    },
    Class {
        name: String,
        decls: Vec<&'a Tree<'a>>,
        functions: Vec<&'a Tree<'a>>,
    },
    Decl {
        decl_type: String,
        name: String,
        init: Option<String>,
    },
    Function {
        access: String,
        ret_type: String,
        name: String,
        decls: Vec<&'a Tree<'a>>,
        statements: Vec<&'a Tree<'a>>,
    },
    FuncCall {
        name: String,
        ret_val: Option<String>,
    },
    While {
        condition: Vec<&'a Tree<'a>>,
        op: OpeType,
        statement: &'a Tree<'a>,
    },
    DoWhile {
        condition: Vec<&'a Tree<'a>>,
        op: OpeType,
        statement: &'a Tree<'a>,
    },
    If {
        condition: Vec<&'a Tree<'a>>,
        op: OpeType,
        then_body: &'a Tree<'a>,
        else_body: Option<&'a Tree<'a>>,
    },
    Switch {
        cases: Vec<SwitchLabel<'a>>,
    },
    Statements(Vec<&'a Tree<'a>>),
    Return(Option<String>),
    Literal(String),
    TokenGet(String),
    EmbedCode(String),
    ExpName(String),
    TermName(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpeType {
    EqualEqual,
    NotEqual,
}

#[derive(Debug, Clone)]
pub struct SwitchLabel<'a> {
    pub labels: Vec<&'a Tree<'a>>,
    pub statement: &'a Tree<'a>,
}
