extern crate cbindgen;

use std::env;

use cbindgen::Language;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
      .with_crate(crate_dir)
      .with_language(Language::C)
      .generate()
      .map_or_else(
        |error| match error {
            e@cbindgen::Error::ParseSyntaxError { .. } => { eprintln!("Parse error: {:?}", e) }
            e => panic!("{:?}", e),
        },
        |bindings| {
            bindings.write_to_file("headers/wm_macos.h");
        },
    );
}