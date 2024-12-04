extern crate cbindgen;

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::generate(crate_dir)
//      .expect("Unable to generate bindings")
//      .write_to_file("headers/kwm_macos.h");
      .map_or_else(
        |error| match error {
            e@cbindgen::Error::ParseSyntaxError { .. } => { eprintln!("Parse error: {:?}", e) }
            e => panic!("{:?}", e),
        },
        |bindings| {
            bindings.write_to_file("headers/kwm_macos.h");
        },
    );
    unsafe {
        env::set_var("RUSTC_BOOTSTRAP", "0");
    }
}