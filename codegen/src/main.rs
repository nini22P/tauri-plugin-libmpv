use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_root = manifest_dir
        .parent()
        .expect("Failed to find parent directory");
    let src_dir = project_root.join("src");

    let header_path = src_dir.join("libmpv_wrapper.h");
    let bindings_path = src_dir.join("wrapper.rs");

    let header_url =
        "https://raw.githubusercontent.com/nini22P/libmpv-wrapper/main/include/libmpv_wrapper.h";

    println!("Downloading header from {}...", header_url);
    let header_content = reqwest::blocking::get(header_url)
        .expect("Failed to download header")
        .text()
        .expect("Failed to get text content");

    fs::write(&header_path, &header_content).expect("Failed to write header file");
    println!("Header saved to: {}", header_path.display());

    println!("Generating bindings...");
    let bindings = bindgen::Builder::default()
        .raw_line("/* Run 'cargo run -p codegen' to regenerate */")
        .raw_line("\n")
        .raw_line("#![allow(unsafe_op_in_unsafe_fn)]")
        .header(header_path.to_str().unwrap())
        .dynamic_library_name("LibmpvWrapper")
        .dynamic_link_require_all(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .formatter(bindgen::Formatter::Prettyplease)
        .allowlist_function("mpv_wrapper_.*")
        .allowlist_type("Mpv")
        .allowlist_type("EventCallback")
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(&bindings_path)
        .expect("Couldn't write bindings!");

    fs::remove_file(&header_path).expect("Failed to remove temporary header file");

    println!(
        "Successfully updated bindings at: {}",
        bindings_path.display()
    );
}
