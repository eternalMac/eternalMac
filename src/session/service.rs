pub fn pin(mut existing: Vec<String>, name: &str) -> Vec<String> {
    if existing.iter().any(|candidate| candidate == name) {
        return existing;
    }
    existing.push(name.to_string());
    existing
}

pub fn unpin(existing: Vec<String>, name: &str) -> Vec<String> {
    existing
        .into_iter()
        .filter(|candidate| candidate != name)
        .collect()
}
