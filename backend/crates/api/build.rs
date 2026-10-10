use std::process::Command;
fn git(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}
fn main() {
    println!("cargo:rerun-if-env-changed=AIHUB_BUILD_REVISION");
    println!("cargo:rerun-if-changed=../../../../.git/HEAD");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../../migrations");
    let mut migrations: Vec<String> = std::fs::read_dir("../../migrations")
        .expect("own migrations directory required")
        .map(|entry| {
            entry
                .expect("migration entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".sql"))
        .collect();
    migrations.sort();
    let schema = migrations
        .last()
        .expect("at least one own migration")
        .trim_end_matches(".sql");
    println!("cargo:rustc-env=AIHUB_SCHEMA_REVISION={schema}");
    let revision =
        std::env::var("AIHUB_BUILD_REVISION").unwrap_or_else(|_| git(&["rev-parse", "HEAD"]));
    let dirty = !git(&["status", "--porcelain"]).is_empty();
    println!("cargo:rustc-env=AIHUB_SOURCE_REVISION={revision}");
    println!("cargo:rustc-env=AIHUB_SOURCE_DIRTY={dirty}");
}
