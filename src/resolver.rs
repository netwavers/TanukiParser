use crate::ast::{EBNFInfo, Node, NodeKind};

// In Rust, instead of recursive visitor mutation, it's often better 
// to build a symbol table first.
pub fn resolve_references<'a>(info: &mut EBNFInfo<'a>) -> anyhow::Result<()> {
    let rule_names: Vec<String> = info.rules.iter().filter_map(|r| {
        if let NodeKind::Rule { name: rname, .. } = &r.kind {
            Some(rname.clone())
        } else {
            None
        }
    }).collect();

    let token_names: Vec<String> = info.tokens.iter().filter_map(|t| {
        if let NodeKind::TokenDef { name: tname, .. } = &t.kind {
            Some(tname.clone())
        } else {
            None
        }
    }).collect();

    for rule in &info.rules {
        check_node(rule, &rule_names, &token_names);
    }

    Ok(())
}

fn check_node<'a>(node: &Node<'a>, rules: &[String], tokens: &[String]) {
    match &node.kind {
        NodeKind::Rule { elements, .. } => check_node(elements, rules, tokens),
        NodeKind::Elements(inner) => check_node(inner, rules, tokens),
        NodeKind::Alternation(list) | NodeKind::Concatenation(list) | NodeKind::RuleList(list) => {
            for child in list {
                check_node(child, rules, tokens);
            }
        }
        NodeKind::Repetition { element, repeat } => {
            check_node(element, rules, tokens);
            if let Some(r) = repeat {
                check_node(r, rules, tokens);
            }
        }
        NodeKind::Element(inner) | NodeKind::Group(inner) | NodeKind::Option(inner) => check_node(inner, rules, tokens),
        NodeKind::RuleName { name, .. } => {
            if !rules.contains(name) && !tokens.contains(name) {
                println!("  Warning: Undefined rule or token: {} at line {}, col {}", name, node.span.line, node.span.column);
            }
        }
        _ => {}
    }
}
