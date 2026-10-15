#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
mod monitor;
#[cfg(target_os = "macos")]
mod updater;

fn main() {
    #[cfg(target_os = "macos")]
    mac::run();
    #[cfg(not(target_os = "macos"))]
    eprintln!("Dunkerfy requires macOS 13 or later.");
}
