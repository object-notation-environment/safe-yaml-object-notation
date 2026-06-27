use std::{env, fs, process};

use syon_parser::{parse_document, Value};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: syon <file.syon>");
        process::exit(1);
    }

    let src = fs::read_to_string(&args[1]).unwrap_or_else(|e| {
        eprintln!("error reading {}: {e}", args[1]);
        process::exit(1);
    });

    match parse_document(&src) {
        Ok(doc) => println!("{}", to_json(&doc.body, 0)),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn to_json(value: &Value, depth: usize) -> String {
    let pad = "  ".repeat(depth);
    let inner = "  ".repeat(depth + 1);
    match value {
        Value::Scalar(s) => serde_json::to_string(s).unwrap(),
        Value::LiteralBlock(s) => serde_json::to_string(s).unwrap(),
        Value::Mapping(m) => {
            if m.entries.is_empty() {
                return "{}".into();
            }
            let pairs: Vec<String> = m
                .entries
                .iter()
                .map(|e| {
                    format!(
                        "{inner}{}: {}",
                        serde_json::to_string(&e.key).unwrap(),
                        to_json(&e.value, depth + 1)
                    )
                })
                .collect();
            format!("{{\n{}\n{pad}}}", pairs.join(",\n"))
        }
        Value::Sequence(seq) => {
            if seq.items.is_empty() {
                return "[]".into();
            }
            let elems: Vec<String> = seq
                .items
                .iter()
                .map(|v| format!("{inner}{}", to_json(v, depth + 1)))
                .collect();
            format!("[\n{}\n{pad}]", elems.join(",\n"))
        }
    }
}
