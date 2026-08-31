fn main() {
    let args: std::vec::Vec<std::string::String> = std::env::args().collect();
    let input  = args.get(1).map(|s| s.as_str()).unwrap_or("examples/calendar.yml");
    let output = args.get(2).map(|s| s.as_str()).unwrap_or("src/dsl_compiled.rs");
    let src = std::fs::read(input)
        .unwrap_or_else(|e| { eprintln!("read error: {e}"); std::process::exit(1); });
    let store_ids = &["File", "Memory"];
    context_engine::dsl::Dsl::write(&src, store_ids, output)
        .unwrap_or_else(|e| { eprintln!("compile error: {e}"); std::process::exit(1); });
    println!("written: {}", output);
}
