#[cfg(target_os = "macos")]
mod macos;
mod common;
mod logger;
mod logger_api;

#[cfg(target_os = "macos")]
fn main() {
    use macos::playground::playground::run;
    
    run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}
