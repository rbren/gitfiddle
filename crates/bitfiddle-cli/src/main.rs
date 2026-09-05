//! Headless bitfiddle CLI (PRD §5.5).
//!
//! Usage:
//!   bitfiddle render <rack.bitfiddle.yaml> <out.wav> [--seconds N]
//!   bitfiddle validate <rack.bitfiddle.yaml>
//!   bitfiddle modules

use std::path::Path;
use std::process::ExitCode;

use bitfiddle_engine::modules::builtins::{all_type_ids, spec};
use bitfiddle_engine::render::render_to_wav;
use bitfiddle_engine::validate::validate_document;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("render") => cmd_render(&args[1..]),
        Some("validate") => cmd_validate(&args[1..]),
        Some("modules") => cmd_modules(),
        _ => {
            eprintln!("usage:");
            eprintln!("  bitfiddle render <rack.bitfiddle.yaml> <out.wav> [--seconds N]");
            eprintln!("  bitfiddle validate <rack.bitfiddle.yaml>");
            eprintln!("  bitfiddle modules");
            ExitCode::from(2)
        }
    }
}

fn cmd_modules() -> ExitCode {
    for id in all_type_ids() {
        let s = spec(id).unwrap();
        let ins: Vec<String> = s
            .inputs
            .iter()
            .map(|p| format!("{}:{}", p.id, p.signal.name()))
            .collect();
        let outs: Vec<String> = s
            .outputs
            .iter()
            .map(|p| format!("{}:{}", p.id, p.signal.name()))
            .collect();
        println!(
            "{:<18} {:<22} {:?}  {}x{}u  in[{}] out[{}]",
            id,
            s.name,
            s.category(),
            s.width_units,
            s.height_units,
            ins.join(", "),
            outs.join(", ")
        );
    }
    ExitCode::SUCCESS
}

fn cmd_render(args: &[String]) -> ExitCode {
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        eprintln!("render: expected <rack.bitfiddle.yaml> <out.wav>");
        return ExitCode::from(2);
    };
    let mut seconds = 2.0f64;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--seconds" {
            match args.get(i + 1).and_then(|s| s.parse().ok()) {
                Some(v) => seconds = v,
                None => {
                    eprintln!("render: --seconds requires a number");
                    return ExitCode::from(2);
                }
            }
            i += 2;
        } else {
            eprintln!("render: unknown argument {}", args[i]);
            return ExitCode::from(2);
        }
    }

    let text = match std::fs::read_to_string(input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("render: cannot read {input}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let doc = match validate_document(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("render: invalid rack: {e}");
            return ExitCode::FAILURE;
        }
    };
    match render_to_wav(doc, seconds, Path::new(output)) {
        Ok(()) => {
            println!("rendered {seconds}s to {output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("render: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_validate(args: &[String]) -> ExitCode {
    let Some(input) = args.first() else {
        eprintln!("validate: expected <rack.bitfiddle.yaml>");
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("validate: cannot read {input}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match validate_document(&text) {
        Ok(doc) => {
            println!(
                "valid: {} ({} modules, {} wires)",
                doc.rack.name,
                doc.modules.len(),
                doc.wires.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("invalid: {e}");
            ExitCode::FAILURE
        }
    }
}
