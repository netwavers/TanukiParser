pub mod ast;
pub mod tokenizer;
pub mod parser;
pub mod resolver;
pub mod node_parser;
pub mod generator;
pub mod python_generator;
pub mod generated_parser;

use clap::{Parser, ValueEnum};
use std::fs::File;
use std::io::BufWriter;
use crate::tokenizer::Tokenizer;
use crate::parser::EBNFParser;
use crate::node_parser::NodeParser;
use crate::generator::get_generator;
use crate::resolver::resolve_references;
use crate::ast::TargetLanguage;
use typed_arena::Arena;

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Target {
    Csharp,
    Rust,
    Python,
}

impl From<Target> for TargetLanguage {
    fn from(t: Target) -> Self {
        match t {
            Target::Csharp => TargetLanguage::CSharp,
            Target::Rust => TargetLanguage::Rust,
            Target::Python => TargetLanguage::Python,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input EBNF file path
    #[arg(short, long)]
    input: String,

    /// Output file path
    #[arg(short, long)]
    output: Option<String>,

    /// Target language
    #[arg(short, long, value_enum, default_value_t = Target::Csharp)]
    target: Target,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    println!("TanukiParser Processing...");
    println!("  Reading: {}", args.input);
    println!("  Target: {:?}", args.target);

    let node_arena = Arena::new();
    let tokenizer = Tokenizer::new(&args.input)?;
    let mut parser = EBNFParser::new(tokenizer, &node_arena);
    
    let info = parser.parse();
    
    println!("DEBUG: info exists: {}, errors count: {}", info.is_some(), parser.errors.len());

    if !parser.errors.is_empty() {
        println!("Errors found during parsing:");
        for diag in &parser.errors {
            println!("  [{}:{}] {}", diag.span.line, diag.span.column, diag.message);
        }
        return Err(anyhow::anyhow!("Parsing failed with {} errors", parser.errors.len()));
    }

    let mut info = info.ok_or_else(|| anyhow::anyhow!("Failed to parse EBNF definition"))?;

    println!("  Grammar parsed successfully.");
    println!("  Namespace: {}", info.namespace);
    println!("  ClassName: {}", info.class_name);
    println!("  Rules count: {}", info.rules.len());

    // Resolve references
    resolve_references(&mut info)?;
    println!("  References resolved.");

    // Convert Grammar AST to Code AST
    let tree_arena = Arena::new();
    let node_parser = NodeParser::new(info, &tree_arena);
    let code_tree = node_parser.parse();
    println!("  Code AST generated.");

    // Generate Code
    let target_lang: TargetLanguage = args.target.into();
    let default_ext = match target_lang {
        TargetLanguage::CSharp => ".cs",
        TargetLanguage::Rust => ".rs",
        TargetLanguage::Python => ".py",
    };
    
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = "GeneratedParser".to_string();
        path.push_str(default_ext);
        path
    });
    
    let out_file = File::create(&output_path)?;
    let mut writer = BufWriter::new(out_file);
    
    let mut generator = get_generator(target_lang);
    generator.generate(code_tree, &mut writer)?;
    
    println!("  Success! Output written to: {}", output_path);

    Ok(())
}
