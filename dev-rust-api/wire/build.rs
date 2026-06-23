//! # Contract Versioning Rules (SemVer)
//!
//! Contract Version: X.Y.Z
//! - **MAJOR (X)**: Incremented on breaking C-ABI layout changes (e.g., changing fields,
//!   sizes, or alignments in existing structures).
//! - **MINOR (Y)**: Incremented when adding new structures without modifying existing ones.
//! - **PATCH (Z)**: Incremented on internal optimizations that do not affect binary layout.


use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn translate_type(rust_type: &str) -> String {
    let rust_type = rust_type.trim();
    match rust_type {
        "u8" => "ctypes.c_uint8".to_string(),
        "i8" => "ctypes.c_int8".to_string(),
        "u16" => "ctypes.c_uint16".to_string(),
        "i16" => "ctypes.c_int16".to_string(),
        "u32" => "ctypes.c_uint32".to_string(),
        "i32" => "ctypes.c_int32".to_string(),
        "u64" => "ctypes.c_uint64".to_string(),
        "i64" => "ctypes.c_int64".to_string(),
        other => {
            // Support arrays [T; N]
            if other.starts_with('[') && other.ends_with(']') {
                let inner = &other[1..other.len() - 1];
                let parts: Vec<&str> = inner.split(';').collect();
                if parts.len() == 2 {
                    let elem_type = parts[0].trim();
                    let len_str = parts[1].trim();
                    let mapped_elem = translate_type(elem_type);
                    return format!("({} * {})", mapped_elem, len_str);
                }
            }
            // Shift-Left Validation: panic on unknown type
            panic!("Shift-Left Validation: Unknown/unsupported C-ABI type '{}'", other);
        }
    }
}

fn parse_fields(fields_text: &str) -> Vec<(String, String)> {
    // Remove block comments /* ... */
    let re_block_comment = regex::Regex::new(r"(?s)/\*.*?\*/").unwrap();
    let fields_text = re_block_comment.replace_all(fields_text, "");
    
    // Remove line comments // ...
    let re_line_comment = regex::Regex::new(r"//[^\n]*").unwrap();
    let fields_text = re_line_comment.replace_all(&fields_text, "");

    let mut fields = Vec::new();
    for segment in fields_text.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        
        // Find colon (first ':' that is not part of '::')
        let mut colon_idx = None;
        let bytes = segment.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b':' {
                if i + 1 < bytes.len() && bytes[i + 1] == b':' {
                    i += 2;
                    continue;
                }
                if i > 0 && bytes[i - 1] == b':' {
                    i += 1;
                    continue;
                }
                colon_idx = Some(i);
                break;
            }
            i += 1;
        }

        if let Some(idx) = colon_idx {
            let name_part = segment[..idx].trim();
            let type_part = segment[idx + 1..].trim();
            
            // Remove attributes #[...] from name_part
            let re_attr = regex::Regex::new(r"(?s)#\[.*?\]").unwrap();
            let name_part_cleaned = re_attr.replace_all(name_part, "");
            
            if let Some(field_name) = name_part_cleaned.split_whitespace().last() {
                fields.push((field_name.to_string(), type_part.to_string()));
            }
        }
    }
    fields
}

struct StructDef {
    name: String,
    body: String,
    fields: Vec<(String, String)>,
}

fn main() {
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=../layout/src/");

    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());

    // Find source files
    let mut rs_files = Vec::new();
    if let Ok(entries) = fs::read_dir("src") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rs") {
                    rs_files.push(path);
                }
            }
        }
    }
    if let Ok(entries) = fs::read_dir("../layout/src") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rs") {
                    rs_files.push(path);
                }
            }
        }
    }

    let re_struct = regex::Regex::new(
        r"(?s)(?:crate::)?decl_wire!\s*\{\s*(?P<body>[^}]*?struct\s+(?P<name>[a-zA-Z0-9_]+)\s*\{(?P<fields>[^}]*?)\})\s*\}"
    ).unwrap();

    let mut structs = Vec::new();

    for file_path in rs_files {
        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("Failed to read file: {:?}", file_path));

        for cap in re_struct.captures_iter(&content) {
            let name = cap.name("name").unwrap().as_str().to_string();
            let body = cap.name("body").unwrap().as_str().to_string();
            let fields_raw = cap.name("fields").unwrap().as_str();
            let fields = parse_fields(fields_raw);
            structs.push(StructDef { name, body, fields });
        }
    }

    // Generate temporary dumper.rs
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dumper_rs_path = Path::new(&out_dir).join("dumper.rs");
    
    #[cfg(target_os = "windows")]
    let dumper_bin_path = Path::new(&out_dir).join("dumper.exe");
    #[cfg(not(target_os = "windows"))]
    let dumper_bin_path = Path::new(&out_dir).join("dumper");

    let mut dumper_code = String::new();
    dumper_code.push_str("#![allow(dead_code)]\n\n");

    for s in &structs {
        dumper_code.push_str(&format!("#[repr(C)]\n{}\n\n", s.body));
    }

    dumper_code.push_str("fn main() {\n");
    dumper_code.push_str("    println!(\"# Generated automatically by wire/build.rs. Do not edit manually.\");\n");
    dumper_code.push_str("    println!(\"# ==============================================================================\");\n");
    dumper_code.push_str("    println!(\"# CONTRACT VERSIONING RULES (SemVer):\");\n");
    dumper_code.push_str(&format!("    println!(\"# Contract Version: {}\");\n", version));
    dumper_code.push_str("    println!(\"# - MAJOR (X): Incremented on breaking C-ABI layout changes (e.g., changing fields,\");\n");
    dumper_code.push_str("    println!(\"#             sizes, or alignments in existing structures).\");\n");
    dumper_code.push_str("    println!(\"# - MINOR (Y): Incremented when adding new structures without modifying existing ones.\");\n");
    dumper_code.push_str("    println!(\"# - PATCH (Z): Incremented on internal optimizations that do not affect binary layout.\");\n");
    dumper_code.push_str("    println!(\"# ==============================================================================\");\n");
    dumper_code.push_str("    println!();\n");
    dumper_code.push_str("    println!(\"import ctypes\");\n");
    dumper_code.push_str("    println!();\n");
    dumper_code.push_str(&format!("    println!(\"__version__ = \\\"{}\\\"\");\n", version));
    dumper_code.push_str("    println!();\n");

    for s in &structs {
        dumper_code.push_str(&format!("    println!(\"class {}(ctypes.LittleEndianStructure):\");\n", s.name));
        dumper_code.push_str(&format!("    println!(\"    _pack_ = {{}}\", std::mem::align_of::<{}>());\n", s.name));
        dumper_code.push_str("    println!(\"    _fields_ = [\");\n");
        for (field_name, field_type) in &s.fields {
            let ctypes_type = translate_type(field_type);
            dumper_code.push_str(&format!(
                "    println!(\"        (\\\"{}\\\", {}),\");\n",
                field_name, ctypes_type
            ));
        }
        dumper_code.push_str("    println!(\"    ]\");\n");
        dumper_code.push_str("    println!();\n");
    }

    dumper_code.push_str("}\n");

    fs::write(&dumper_rs_path, dumper_code).expect("Failed to write dumper.rs");

    // Compile dumper.rs
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let status = Command::new(rustc)
        .arg(&dumper_rs_path)
        .arg("-o")
        .arg(&dumper_bin_path)
        .status()
        .expect("Failed to run rustc");

    if !status.success() {
        panic!("Failed to compile dumper.rs");
    }

    // Run binary and capture output
    let output = Command::new(&dumper_bin_path)
        .output()
        .expect("Failed to execute dumper binary");

    if !output.status.success() {
        panic!("Dumper binary failed with exit status");
    }

    let python_code = String::from_utf8(output.stdout).expect("Invalid UTF-8 from dumper stdout");

    // Write to axipy/contract/contract_generated.py in both dev-rust-api and dev-sdk-api
    let paths = [
        Path::new("..").join("axipy").join("contract"),
        Path::new("..").join("..").join("dev-sdk-api").join("axipy").join("contract"),
    ];

    for dest_dir in &paths {
        fs::create_dir_all(dest_dir).expect("Failed to create destination directory");
        let dest_file = dest_dir.join("contract_generated.py");
        fs::write(&dest_file, &python_code).expect("Failed to write contract_generated.py");
    }
}
