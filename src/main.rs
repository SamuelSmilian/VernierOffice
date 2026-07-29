use std::env;
use std::fs;
use std::io::Cursor;
use std::process;

use vernier_office::backend::docx::DocxBackend;
use vernier_office::backend::pptx::PptxBackend;
use vernier_office::backend::Backend;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 || args[1] != "compile" {
        eprintln!("Usage: vernier-office compile <input.vo> [--output <path>] [--format <docx|pptx>]");
        process::exit(1);
    }

    let input_path = &args[2];

    let output_path = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let format_opt = args
        .iter()
        .position(|a| a == "--format")
        .and_then(|i| args.get(i + 1))
        .cloned();

    // Determine format from output extension, --format flag, or default to docx
    let format = format_opt.as_deref().unwrap_or_else(|| {
        output_path
            .as_deref()
            .and_then(|p| {
                if p.ends_with(".pptx") {
                    Some("pptx")
                } else if p.ends_with(".docx") {
                    Some("docx")
                } else {
                    None
                }
            })
            .unwrap_or("docx")
    });

    let output_path = output_path.unwrap_or_else(|| {
        let stem = input_path.strip_suffix(".vo").unwrap_or(input_path);
        let ext = if format == "pptx" { ".pptx" } else { ".docx" };
        format!("{}{}", stem, ext)
    });

    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_path, e);
            process::exit(1);
        }
    };

    let tokens = vernier_office::lexer::lex(&source);
    let document = match vernier_office::parser::parse(tokens) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            process::exit(1);
        }
    };

    let mut buf = Cursor::new(Vec::new());

    match format {
        "pptx" => {
            let backend = PptxBackend;
            if let Err(e) = backend.compile(&document, &mut buf) {
                eprintln!("Compile error: {}", e);
                process::exit(1);
            }
        }
        _ => {
            let backend = DocxBackend;
            if let Err(e) = backend.compile(&document, &mut buf) {
                eprintln!("Compile error: {}", e);
                process::exit(1);
            }
        }
    }

    match fs::write(&output_path, buf.into_inner()) {
        Ok(_) => eprintln!("Wrote {}", output_path),
        Err(e) => {
            eprintln!("Error writing {}: {}", output_path, e);
            process::exit(1);
        }
    }
}
