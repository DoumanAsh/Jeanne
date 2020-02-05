#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

use std::thread;

#[macro_use]
mod utils;
mod constants;
mod stats;
mod config;
mod discord;
mod twitter;

fn main() {
    config::init();

    thread::Builder::new().name("twitter-worker".to_owned())
                          .spawn(twitter::worker)
                          .expect("To create twitter thread");

    discord::run();

    match crate::config::DISCORD.save() {
        Ok(_) => {
            rogu::info!("Discord config is updated.");
        },
        Err(error) => {
            rogu::error!("Discord unable to save config: {}", error);
        }
    }
}
