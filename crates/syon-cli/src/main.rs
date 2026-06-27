use std::{env, fs, process};

use syon_parser::{ast::Value, parse};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: syon <file.syon>");
        process::exit(1);
    }

    let path = &args[1];
    let src = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error reading {path}: {e}");
        process::exit(1);
    });

    match parse(&src) {
        Ok(value) => println!("{}", value_to_json(&value, 0)),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn value_to_json(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let inner_pad = "  ".repeat(indent + 1);
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Str(s) => format!("{s:?}"),
        Value::Sequence(items) => {
            if items.is_empty() {
                return "[]".into();
            }
            let entries: Vec<String> = items
                .iter()
                .map(|v| format!("{inner_pad}{}", value_to_json(v, indent + 1)))
                .collect();
            format!("[\n{}\n{pad}]", entries.join(",\n"))
        }
        Value::Mapping(pairs) => {
            if pairs.is_empty() {
                return "{}".into();
            }
            let entries: Vec<String> = pairs
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{inner_pad}{}: {}",
                        serde_json::to_string(k).unwrap(),
                        value_to_json(v, indent + 1)
                    )
                })
                .collect();
            format!("{{\n{}\n{pad}}}", entries.join(",\n"))
        }
    }
}
