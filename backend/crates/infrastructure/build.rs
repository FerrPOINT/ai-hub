fn main() {
    // sqlx::migrate! tracks embedded files; a newly added file also needs a Cargo dependency.
    println!("cargo:rerun-if-changed=../../migrations");
}
