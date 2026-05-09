pub fn add_output(name: &str, local: &str, remote: &str) -> String {
    format!("sync {name} {local} {remote}")
}

pub fn add(name: &str, local: &str, remote: &str) {
    println!("{}", add_output(name, local, remote));
}

pub fn list_output() -> &'static str {
    "project"
}

pub fn list() {
    println!("{}", list_output());
}

pub fn status_output() -> &'static str {
    "sync healthy"
}

pub fn status() {
    println!("{}", status_output());
}
