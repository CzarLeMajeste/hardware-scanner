use hardware_scanner::generate_report;

fn main() {
    let compact = std::env::args().any(|arg| arg == "--compact");
    let report = generate_report();

    if compact {
        println!("{}", serde_json::to_string(&report).expect("failed to serialize report"));
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("failed to serialize report")
        );
    }
}
