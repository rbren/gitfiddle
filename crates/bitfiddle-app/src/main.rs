#[cfg(feature = "desktop")]
fn main() {
    bitfiddle_app::run();
}

#[cfg(not(feature = "desktop"))]
fn main() {
    eprintln!("bitfiddle-app was built without the desktop feature");
}
