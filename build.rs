fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/luma-moon.ico")
            .compile()
            .expect("Windows icon resource");
    }
    println!("cargo:rerun-if-changed=assets/luma-moon.ico");
}
