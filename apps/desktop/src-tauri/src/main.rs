//! MCNP Desktop 可执行程序入口。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mcnp_desktop_lib::run;

fn main() {
    if let Err(error) = run() {
        eprintln!("MCNP Desktop failed to start: {error}");
        std::process::exit(1);
    }
}
