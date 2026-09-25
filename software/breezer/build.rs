fn main() {
    println!("cargo:rerun-if-changed=src/ui");
    // Compile the Slint UI into Rust.
    slint_build::compile("src/ui/main_window.slint").expect("failed to compile .slint UI");
}