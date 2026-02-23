use std::path::Path;

fn main() {
    // Try loading a config file, fall back to defaults
    let cfg = match config::Config::load(Path::new("app.conf")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: could not load config: {e}");
            let mut cfg = config::Config::default();
            cfg.set("name", config::Value::Str("my-app".into()));
            cfg.set("port", config::Value::Int(8080));
            cfg.set("debug", config::Value::Bool(false));
            cfg
        }
    };

    println!("App: {}", cfg.get_str("name"));
    println!("Port: {}", cfg.get_int("port"));
    println!("Debug: {}", cfg.get_bool("debug"));
    println!("Total settings: {}", cfg.len());

    // Demonstrate iteration (added by patch 003)
    println!("\nAll settings:");
    for (key, value) in cfg.iter() {
        println!("  {key} = {value}");
    }
}
