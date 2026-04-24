use crate::ast::{EBNFInfo, Node, NodeKind, Tree, TreeKind, OpeType, SwitchLabel, Span};
use typed_arena::Arena;

pub struct NodeParser<'a> {
    info: EBNFInfo<'a>,
    arena: &'a Arena<Tree<'a>>,
}

impl<'a> NodeParser<'a> {
    pub fn new(info: EBNFInfo<'a>, arena: &'a Arena<Tree<'a>>) -> Self {
        Self { info, arena }
    }

    pub fn parse(mut self) -> &'a Tree<'a> {
        let rules = self.info.rules.clone();
        self.visit_rulelist(&rules)
    }

    fn visit_rulelist(&mut self, rules: &[&'a Node<'a>]) -> &'a Tree<'a> {
        let mut t_decls: Vec<&'a Tree<'a>> = Vec::new();
        for d in &self.info.decls {
            if let NodeKind::Decl { decl_type, name, init } = &d.kind {
                t_decls.push(self.arena.alloc(Tree {
                    kind: TreeKind::Decl {
                        decl_type: decl_type.clone(),
                        name: name.clone(),
                        init: init.clone(),
                    },
                    span: d.span,
                }));
            }
        }

        let mut functions: Vec<&'a Tree<'a>> = Vec::new();
        for rule in rules {
            functions.push(self.visit_rule(rule));
        }

        // Add 'parse' main function
        if let Some(first_rule) = rules.first() {
            if let NodeKind::Rule { name: rname, .. } = &first_rule.kind {
                let main_func = Tree {
                    kind: TreeKind::Function {
                        access: "public".into(),
                        ret_type: "Node".into(),
                        name: "parse".into(),
                        decls: vec![self.arena.alloc(Tree {
                            kind: TreeKind::Decl {
                                decl_type: "Node".into(),
                                name: "ret_val".into(),
                                init: Some("null".into()),
                            },
                            span: first_rule.span,
                        })],
                        statements: vec![
                            self.arena.alloc(Tree {
                                kind: TreeKind::FuncCall {
                                    name: rname.clone(),
                                    ret_val: Some("ret_val".into()),
                                },
                                span: first_rule.span,
                            }),
                            self.arena.alloc(Tree {
                                kind: TreeKind::Return(Some("ret_val".into())),
                                span: first_rule.span,
                            }),
                        ],
                    },
                    span: first_rule.span,
                };
                functions.push(self.arena.alloc(main_func));
            }
        }

        self.arena.alloc(Tree {
            kind: TreeKind::Namespace {
                name: self.info.namespace.clone(),
                tokens: self.info.tokens.iter().filter_map(|t| if let NodeKind::TokenDef { name: tname, .. } = &t.kind { Some(tname.clone()) } else { None }).collect(),
                class_body: self.arena.alloc(Tree {
                    kind: TreeKind::Class {
                        name: self.info.class_name.clone(),
                        decls: t_decls,
                        functions: functions.into_iter().map(|f| f as &'a Tree<'a>).collect(),
                    },
                    span: Span::default(),
                }),
            },
            span: Span::default(),
        })
    }

    fn visit_rule(&mut self, node: &'a Node<'a>) -> &'a Tree<'a> {
        if let NodeKind::Rule { name, elements } = &node.kind {
            let mut func_decls: Vec<&'a Tree<'a>> = vec![self.arena.alloc(Tree {
                kind: TreeKind::Decl {
                    decl_type: "Node".into(),
                    name: "ret_val".into(),
                    init: Some("null".into()),
                },
                span: node.span,
            })];
            
            let statement = self.visit_node(elements, &mut func_decls);
            let mut statements = vec![statement];
            statements.push(self.arena.alloc(Tree {
                kind: TreeKind::Return(Some("ret_val".into())),
                span: node.span,
            }));

            self.arena.alloc(Tree {
                kind: TreeKind::Function {
                    access: "".into(), // default
                    ret_type: "Node".into(),
                    name: name.clone(),
                    decls: func_decls,
                    statements: statements.into_iter().map(|s| s as &Tree).collect(),
                },
                span: node.span,
            })
        } else {
            self.arena.alloc(Tree {
                kind: TreeKind::Literal("// Error: Not a rule".into()),
                span: node.span,
            })
        }
    }

    fn visit_node(&mut self, node: &'a Node<'a>, cur_func_decls: &mut Vec<&'a Tree<'a>>) -> &'a Tree<'a> {
        let inner_tree = match &node.kind {
            NodeKind::Elements(inner) => self.visit_node(inner, cur_func_decls),
            NodeKind::Alternation(list) => {
                if list.len() == 1 {
                    self.visit_node(list[0], cur_func_decls)
                } else {
                    let mut cases = Vec::new();
                    for child in list {
                        cases.push(SwitchLabel {
                            labels: self.get_terms(child),
                            statement: self.visit_node(child, cur_func_decls),
                        });
                    }
                    self.arena.alloc(Tree {
                        kind: TreeKind::Switch { cases },
                        span: node.span,
                    })
                }
            }
            NodeKind::Concatenation(list) => {
                let stmts: Vec<&'a Tree<'a>> = list.iter().map(|n| self.visit_node(n, cur_func_decls) as &'a Tree<'a>).collect();
                self.arena.alloc(Tree {
                    kind: TreeKind::Statements(stmts),
                    span: node.span,
                })
            }
            NodeKind::Repetition { element, repeat } => {
                if let Some(rep) = repeat {
                    let terms = self.get_terms(element);
                    match &rep.kind {
                        NodeKind::Repeat0 => self.arena.alloc(Tree {
                            kind: TreeKind::While {
                                condition: terms,
                                op: OpeType::EqualEqual,
                                statement: self.visit_node(element, cur_func_decls),
                            },
                            span: node.span,
                        }),
                        NodeKind::Repeat1 => self.arena.alloc(Tree {
                            kind: TreeKind::DoWhile {
                                condition: terms,
                                op: OpeType::EqualEqual,
                                statement: self.visit_node(element, cur_func_decls),
                            },
                            span: node.span,
                        }),
                        _ => self.visit_node(element, cur_func_decls),
                    }
                } else {
                    self.visit_node(element, cur_func_decls)
                }
            }
            NodeKind::Option(inner) => {
                let terms = self.get_terms(inner);
                self.arena.alloc(Tree {
                    kind: TreeKind::If {
                        condition: terms,
                        op: OpeType::EqualEqual,
                        then_body: self.visit_node(inner, cur_func_decls),
                        else_body: None,
                    },
                    span: node.span,
                })
            }
            NodeKind::Group(inner) => self.visit_node(inner, cur_func_decls),
            NodeKind::Element(inner) => self.visit_node(inner, cur_func_decls),
            NodeKind::RuleName { name, alias } => {
                if self.is_token(name) {
                    let mut stmts: Vec<&'a Tree<'a>> = vec![
                        self.arena.alloc(Tree {
                            kind: TreeKind::If {
                                condition: vec![self.arena.alloc(Tree {
                                    kind: TreeKind::TermName(name.clone()),
                                    span: node.span,
                                })],
                                op: OpeType::NotEqual,
                                then_body: self.arena.alloc(Tree {
                                    kind: TreeKind::Return(Some("null".into())),
                                    span: node.span,
                                }),
                                else_body: None,
                            },
                            span: node.span,
                        })
                    ];

                    if !node.embed_code.is_empty() {
                        stmts.push(self.arena.alloc(Tree {
                            kind: TreeKind::EmbedCode(node.embed_code.clone()),
                            span: node.span,
                        }));
                    }

                    stmts.push(self.arena.alloc(Tree {
                        kind: TreeKind::TokenGet("tokenizer".into()),
                        span: node.span,
                    }));

                    let mut tree = self.arena.alloc(Tree {
                        kind: TreeKind::Statements(stmts),
                        span: node.span,
                    });
                    
                    // アクションは既に処理したので、visit_node の最後で二重に追加されないようにする
                    // (ただし、Node構造体自体は不変なので、このツリーを返せば良い)
                    return tree;
                } else {
                    let ret_val_name = format!("ret_val_{}", alias);
                    let ret_val_exists = cur_func_decls.iter().any(|d| {
                        if let TreeKind::Decl { name: dname, .. } = &d.kind { dname == &ret_val_name } else { false }
                    });
                    if !ret_val_exists {
                        cur_func_decls.push(self.arena.alloc(Tree {
                            kind: TreeKind::Decl {
                                decl_type: "Node".into(),
                                name: ret_val_name.clone(),
                                init: Some("null".into()),
                            },
                            span: node.span,
                        }));
                    }
                    let rule_exists = self.info.rules.iter().any(|r| {
                        if let NodeKind::Rule { name: rname, .. } = &r.kind { rname == name } else { false }
                    });

                    if rule_exists {
                        self.arena.alloc(Tree {
                            kind: TreeKind::FuncCall {
                                name: name.clone(),
                                ret_val: Some(ret_val_name),
                            },
                            span: node.span,
                        })
                    } else {
                        self.arena.alloc(Tree {
                            kind: TreeKind::Literal(format!("// Warning: Undefined rule call: {}();", name)),
                            span: node.span,
                        })
                    }
                }
            }
            NodeKind::CharVal(s) => self.arena.alloc(Tree {
                kind: TreeKind::Literal(format!("// literal: {}", s)),
                span: node.span,
            }),
            _ => self.arena.alloc(Tree {
                kind: TreeKind::Literal("// Other node".into()),
                span: node.span,
            }),
        };
        
        if !node.embed_code.is_empty() {
            self.arena.alloc(Tree {
                kind: TreeKind::Statements(vec![
                    inner_tree,
                    self.arena.alloc(Tree {
                        kind: TreeKind::EmbedCode(node.embed_code.clone()),
                        span: node.span,
                    }),
                ]),
                span: node.span,
            })
        } else {
            inner_tree
        }
    }

    fn is_token(&self, name: &str) -> bool {
        self.info.tokens.iter().any(|t| {
            if let NodeKind::TokenDef { name: tname, .. } = &t.kind {
                tname == name
            } else {
                false
            }
        })
    }

    fn get_terms(&self, node: &'a Node<'a>) -> Vec<&'a Tree<'a>> {
        match &node.kind {
            NodeKind::Element(inner) => self.get_terms(inner),
            NodeKind::RuleName { name, .. } => {
                if self.is_token(name) {
                    vec![self.arena.alloc(Tree {
                        kind: TreeKind::TermName(name.clone()),
                        span: node.span,
                    })]
                } else {
                    // Look up rule
                    if let Some(rule) = self.info.rules.iter().find(|r| {
                        if let NodeKind::Rule { name: rname, .. } = &r.kind { rname == name } else { false }
                    }) {
                        if let NodeKind::Rule { elements, .. } = &rule.kind {
                            self.get_terms(elements)
                        } else {
                            vec![self.arena.alloc(Tree {
                                kind: TreeKind::ExpName(name.clone()),
                                span: node.span,
                            })]
                        }
                    } else {
                        vec![self.arena.alloc(Tree {
                            kind: TreeKind::ExpName(name.clone()),
                            span: node.span,
                        })]
                    }
                }
            }
            NodeKind::Alternation(list) => {
                let mut terms = Vec::new();
                for child in list {
                    terms.extend(self.get_terms(child));
                }
                terms
            }
            NodeKind::Concatenation(list) => {
                if let Some(first) = list.first() {
                    self.get_terms(first)
                } else {
                    vec![]
                }
            }
            NodeKind::Repetition { element, .. } => self.get_terms(element),
            NodeKind::Option(inner) | NodeKind::Group(inner) | NodeKind::Elements(inner) => self.get_terms(inner),
            _ => vec![],
        }
    }
}
