pub fn list() {
    println!("default");
}

pub fn create(name: &str) {
    println!("created {name}");
}

pub fn pin_session(name: &str) {
    println!("pinned {name}");
}

pub fn unpin_session(name: &str) {
    println!("unpinned {name}");
}
