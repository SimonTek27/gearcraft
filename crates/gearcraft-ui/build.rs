fn main() {
    // Project design is dark-only (accent #00E7A0) → force the dark widget
    // style instead of following the system's light/dark setting.
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".to_string());
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint compile failed");
}
