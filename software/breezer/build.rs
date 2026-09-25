fn main() {
    println!("cargo:rerun-if-changed=src/ui");
    println!("cargo:rerun-if-changed=translations");

    // Compile the Slint UI into Rust, bundling gettext .po translations and
    // disabling the default per-component translation context (our .po files
    // therefore use bare msgid/msgstr entries).
    let config = slint_build::CompilerConfiguration::new()
        .with_default_translation_context(slint_build::DefaultTranslationContext::None)
        .with_bundled_translations("translations");
    slint_build::compile_with_config("src/ui/main_window.slint", config)
        .expect("failed to compile .slint UI");
}
