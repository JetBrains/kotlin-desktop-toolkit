#[cfg(target_os = "macos")]
mod macos;
mod common;

#[cfg(target_os = "macos")]
fn main() {
    use macos::application::run;

    run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}
