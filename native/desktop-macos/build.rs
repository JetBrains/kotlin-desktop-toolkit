extern crate cbindgen;

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // Get the crate name from Cargo environment variable

    let artifact_name = env::var("CARGO_PKG_NAME").unwrap().replace("-", "_");
    let dylib_name = format!("lib{artifact_name}.dylib");
    let header_name = format!("{artifact_name}.h");
    println!("cargo:rustc-link-arg=-Wl,-install_name,./{dylib_name}");

    cbindgen::generate(crate_dir)
        //.expect("Unable to generate bindings")
        //.write_to_file("headers/desktop_macos.h");
        .map_or_else(
            |error| match error {
                e @ cbindgen::Error::ParseSyntaxError { .. } => {
                    eprintln!("Parse error: {:?}", e)
                }
                e => panic!("{:?}", e),
            },
            |bindings| {
                bindings.write_to_file(format!("headers/{header_name}"));
            },
        );
    unsafe {
        env::set_var("RUSTC_BOOTSTRAP", "0");
    }
}
