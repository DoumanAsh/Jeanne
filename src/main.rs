#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

mod constants;
mod stats;
mod config;
mod discord;

fn main() {
    config::init();

    discord::run();
}
