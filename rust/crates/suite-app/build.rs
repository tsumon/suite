fn main() {
    let manifest = std::path::Path::new("app.manifest")
        .canonicalize()
        .expect("app.manifest missing");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
