//! Linux evdev backend -- keyboard capture via /dev/input/event*.

mod capture;

pub use capture::LinuxEvdevCapture;
