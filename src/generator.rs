use crate::ast::{Tree, TreeKind, OpeType, TargetLanguage};
use std::io::Write;

pub trait Generator<'a> {
    fn generate(&mut self, tree: &'a Tree<'a>, writer: &mut dyn Write) -> anyhow::Result<()>;
}

pub struct CSharpGenerator {
    ident: usize,
}

impl<'a> CSharpGenerator {
    pub fn new() -> Self {
        Self { ident: 0 }
    }

    fn output(&self, s: &str, writer: &mut dyn Write) {
        let tabs = "    ".repeat(self.ident);
        writeln!(writer, "{}{}", tabs, s).unwrap();
    }

    fn format_condition(&self, conditions: &[&'a Tree<'a>], op: OpeType) -> String {
        let op_str = match op {
            OpeType::EqualEqual => "==",
            OpeType::NotEqual => "!=",
        };
        let join_str = match op {
            OpeType::EqualEqual => " || ",
            OpeType::NotEqual => " && ",
        };

        conditions.iter().map(|c| {
            let name = match &c.kind {
                TreeKind::ExpName(n) | TreeKind::TermName(n) => n.clone(),
                _ => "unknown".to_string(),
            };
            format!("(token.Type {} TokenType.{})", op_str, name)
        }).collect::<Vec<_>>().join(join_str)
    }
}

impl<'a> Generator<'a> for CSharpGenerator {
    fn generate(&mut self, tree: &'a Tree<'a>, writer: &mut dyn Write) -> anyhow::Result<()> {
        match &tree.kind {
            TreeKind::Namespace { name, tokens, class_body } => {
                self.output(&format!("namespace {}", name), writer);
                self.output("{", writer);
                self.ident += 1;
                
                self.output("enum TokenType {", writer);
                self.ident += 1;
                for (i, token) in tokens.iter().enumerate() {
                    let mut line = token.clone();
                    if i < tokens.len() - 1 {
                        line.push(',');
                    }
                    self.output(&line, writer);
                }
                self.ident -= 1;
                self.output("};", writer);
                self.output("", writer);
                
                self.generate(class_body, writer)?;
                
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::Class { name, decls, functions } => {
                self.output(&format!("class {}", name), writer);
                self.output("{", writer);
                self.ident += 1;
                
                for d in decls {
                    self.generate(d, writer)?;
                }
                for f in functions {
                    self.generate(f, writer)?;
                }
                
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::Decl { decl_type, name, init } => {
                let mut s = format!("{} {}", decl_type, name);
                if let Some(i) = init {
                    s.push_str(&format!(" = {}", i));
                }
                s.push(';');
                self.output(&s, writer);
            }
            TreeKind::Function { access, ret_type, name, decls, statements } => {
                let acc = if access.is_empty() { "" } else { " " };
                self.output(&format!("{}{} {} {}()", access, acc, ret_type, name), writer);
                self.output("{", writer);
                self.ident += 1;
                
                for d in decls {
                    self.generate(d, writer)?;
                }
                for s in statements {
                    self.generate(s, writer)?;
                }
                
                self.ident -= 1;
                self.output("}", writer);
                self.output("", writer);
            }
            TreeKind::FuncCall { name, ret_val } => {
                if let Some(rv) = ret_val {
                    self.output(&format!("{} = {}();", rv, name), writer);
                } else {
                    self.output(&format!("{}();", name), writer);
                }
            }
            TreeKind::While { condition, op, statement } => {
                let cond_str = self.format_condition(condition, *op);
                self.output(&format!("while ({})", cond_str), writer);
                self.output("{", writer);
                self.ident += 1;
                self.generate(statement, writer)?;
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::DoWhile { condition, op, statement } => {
                self.output("do {", writer);
                self.ident += 1;
                self.generate(statement, writer)?;
                self.ident -= 1;
                let cond_str = self.format_condition(condition, *op);
                self.output(&format!("}} while ({});", cond_str), writer);
            }
            TreeKind::If { condition, op, then_body, else_body } => {
                let cond_str = self.format_condition(condition, *op);
                self.output(&format!("if ({})", cond_str), writer);
                self.output("{", writer);
                self.ident += 1;
                self.generate(then_body, writer)?;
                self.ident -= 1;
                self.output("}", writer);
                if let Some(eb) = else_body {
                    self.output("else", writer);
                    self.output("{", writer);
                    self.ident += 1;
                    self.generate(eb, writer)?;
                    self.ident -= 1;
                    self.output("}", writer);
                }
            }
            TreeKind::Switch { cases } => {
                self.output("switch (token.Type)", writer);
                self.output("{", writer);
                self.ident += 1;
                for case in cases {
                    for label in &case.labels {
                        if let TreeKind::TermName(name) = &label.kind {
                            self.output(&format!("case TokenType.{}:", name), writer);
                        } else if let TreeKind::ExpName(name) = &label.kind {
                            self.output(&format!("case TokenType.{}:", name), writer);
                        }
                    }
                    self.ident += 1;
                    self.generate(case.statement, writer)?;
                    self.output("break;", writer);
                    self.ident -= 1;
                }
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::Statements(list) => {
                for s in list {
                    self.generate(s, writer)?;
                }
            }
            TreeKind::Return(val) => {
                if let Some(v) = val {
                    self.output(&format!("return {};", v), writer);
                } else {
                    self.output("return;", writer);
                }
            }
            TreeKind::Literal(s) => self.output(s, writer),
            TreeKind::TokenGet(name) => self.output(&format!("token = {}.getToken();", name), writer),
            TreeKind::EmbedCode(code) => {
                let processed = code.replace("$$", "RetVal").replace("$", "RetVal_");
                for line in processed.split(';') {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        self.output(&format!("{};", trimmed), writer);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub struct RustGenerator {
    ident: usize,
}

impl<'a> RustGenerator {
    pub fn new() -> Self {
        Self { ident: 0 }
    }

    fn output(&self, s: &str, writer: &mut dyn Write) {
        let tabs = "    ".repeat(self.ident);
        writeln!(writer, "{}{}", tabs, s).unwrap();
    }

    fn map_token_name(&self, name: &str) -> String {
        match name {
            "RULENAME" => "TRuleName".to_string(),
            "CHAR_VAL" => "TCharVal".to_string(),
            "BIN_VAL" => "TBinVal".to_string(),
            "DEC_VAL" => "TDecVal".to_string(),
            "HEX_VAL" => "THexVal".to_string(),
            "PROSE_VAL" => "TProseVal".to_string(),
            "EQUAL" => "TEqual".to_string(),
            "ALTERNATIVE" => "TAlternative".to_string(),
            "PLUS" => "TPlus".to_string(),
            "ASTERRISK" => "TAsterisk".to_string(),
            "GROUP_LEFT" => "TGroupLeft".to_string(),
            "GROUP_RIGHT" => "TGroupRight".to_string(),
            "OPTION_LEFT" => "TOptionLeft".to_string(),
            "OPTION_RIGHT" => "TOptionRight".to_string(),
            "NL" => "TNl".to_string(),
            _ => name.to_string(),
        }
    }

    fn format_condition(&self, conditions: &[&'a Tree<'a>], op: OpeType) -> String {
        let op_str = match op {
            OpeType::EqualEqual => "==",
            OpeType::NotEqual => "!=",
        };
        let join_str = match op {
            OpeType::EqualEqual => " || ",
            OpeType::NotEqual => " && ",
        };

        let formatted: Vec<String> = conditions.iter().map(|c| {
            let name = match &c.kind {
                TreeKind::ExpName(n) | TreeKind::TermName(n) => n.clone(),
                _ => "UNKNOWN".to_string(),
            };
            format!("(self.token.token_type {} TokenType::{})", op_str, self.map_token_name(&name))
        }).collect();

        if formatted.is_empty() {
            "false".to_string()
        } else {
            formatted.join(join_str)
        }
    }

    fn map_type(&self, t: &str) -> String {
        match t {
            "Node" => "Option<Node<'a>>".to_string(),
            "NodeManager" => "()".to_string(),
            _ => t.to_string(),
        }
    }

    fn map_init(&self, i: &str) -> String {
        match i {
            "null" => "None".to_string(),
            _ => i.to_string(),
        }
    }
}

impl<'a> Generator<'a> for RustGenerator {
    fn generate(&mut self, tree: &'a Tree<'a>, writer: &mut dyn Write) -> anyhow::Result<()> {
        match &tree.kind {
            TreeKind::Namespace { name: _, tokens: _, class_body } => {
                self.output("use crate::ast::{Node, TokenType};", writer);
                self.output("", writer);
                self.generate(class_body, writer)?;
            }
            TreeKind::Class { name, decls, functions } => {
                self.output("#[allow(dead_code, unused_mut, unused_variables, unused_assignments, non_snake_case)]", writer);
                self.output(&format!("pub struct {}<'a> {{", name), writer);
                self.ident += 1;
                self.output("pub tokenizer: &'a mut crate::tokenizer::Tokenizer,", writer);
                self.output("pub token: crate::ast::Token,", writer);
                for d in decls {
                    if let TreeKind::Decl { decl_type, name, .. } = &d.kind {
                        self.output(&format!("pub {}: {},", name.to_lowercase(), self.map_type(decl_type)), writer);
                    }
                }
                self.ident -= 1;
                self.output("}", writer);
                self.output("", writer);

                self.output(&format!("impl<'a> {}<'a> {{", name), writer);
                self.ident += 1;
                
                for f in functions {
                    self.generate(f, writer)?;
                }
                
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::Decl { decl_type, name, init } => {
                let init_val = if let Some(i) = init { self.map_init(i) } else { "None".to_string() };
                self.output(&format!("let mut {} : {} = {};", name, self.map_type(decl_type), init_val), writer);
            }
            TreeKind::Function { access, ret_type, name, decls, statements } => {
                let acc = if access == "public" { "pub " } else { "" };
                self.output(&format!("{}fn {}(&mut self) -> {} {{", acc, name.to_lowercase(), self.map_type(ret_type)), writer);
                self.ident += 1;
                
                for d in decls {
                    self.generate(d, writer)?;
                }
                for s in statements {
                    self.generate(s, writer)?;
                }
                
                self.ident -= 1;
                self.output("}", writer);
                self.output("", writer);
            }
            TreeKind::FuncCall { name, ret_val } => {
                if let Some(rv) = ret_val {
                    self.output(&format!("{} = self.{}();", rv, name.to_lowercase()), writer);
                } else {
                    self.output(&format!("self.{}();", name.to_lowercase()), writer);
                }
            }
            TreeKind::While { condition, op, statement } => {
                let cond_str = self.format_condition(condition, *op);
                self.output(&format!("while {} {{", cond_str), writer);
                self.ident += 1;
                self.generate(statement, writer)?;
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::DoWhile { condition, op, statement } => {
                self.output("loop {", writer);
                self.ident += 1;
                self.generate(statement, writer)?;
                let cond_str = self.format_condition(condition, *op);
                self.output(&format!("if !({}) {{ break; }}", cond_str), writer);
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::If { condition, op, then_body, else_body } => {
                let cond_str = self.format_condition(condition, *op);
                self.output(&format!("if {} {{", cond_str), writer);
                self.ident += 1;
                self.generate(then_body, writer)?;
                self.ident -= 1;
                self.output("}", writer);
                if let Some(eb) = else_body {
                    self.output("else {", writer);
                    self.ident += 1;
                    self.generate(eb, writer)?;
                    self.ident -= 1;
                    self.output("}", writer);
                }
            }
            TreeKind::Switch { cases } => {
                self.output("match self.token.token_type {", writer);
                self.ident += 1;
                for case in cases {
                    let mut labels = Vec::new();
                    for label in &case.labels {
                        if let TreeKind::TermName(name) = &label.kind {
                            labels.push(format!("TokenType::{}", self.map_token_name(name)));
                        } else if let TreeKind::ExpName(name) = &label.kind {
                            labels.push(format!("TokenType::{}", self.map_token_name(name)));
                        }
                    }
                    if labels.is_empty() { continue; }
                    self.output(&format!("{} => {{", labels.join(" | ")), writer);
                    self.ident += 1;
                    self.generate(case.statement, writer)?;
                    self.ident -= 1;
                    self.output("}", writer);
                }
                self.output("_ => {}", writer);
                self.ident -= 1;
                self.output("}", writer);
            }
            TreeKind::Statements(list) => {
                for s in list {
                    self.generate(s, writer)?;
                }
            }
            TreeKind::Return(val) => {
                if let Some(v) = val {
                    self.output(&format!("return {};", self.map_init(v)), writer);
                } else {
                    self.output("return;", writer);
                }
            }
            TreeKind::Literal(s) => {
                if s.starts_with("//") {
                    self.output(s, writer);
                } else {
                    self.output(&self.map_init(s), writer);
                }
            }
            TreeKind::TokenGet(_name) => self.output("self.token = self.tokenizer.get_token();", writer),
            TreeKind::EmbedCode(code) => {
                let processed = code.replace("$$", "ret_val").replace("$", "ret_val_");
                for line in processed.split(';') {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        self.output(&format!("{};", trimmed), writer);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn get_generator<'a>(target: TargetLanguage) -> Box<dyn Generator<'a> + 'a> {
    match target {
        TargetLanguage::CSharp => Box::new(CSharpGenerator::new()),
        TargetLanguage::Rust => Box::new(RustGenerator::new()),
        TargetLanguage::Python => Box::new(crate::python_generator::PythonGenerator::new()),
    }
}
