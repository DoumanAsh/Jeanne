#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

mod constants;
mod stats;
mod config;
mod discord;

fn main() {
    config::init();

    discord::run();

    match crate::config::DISCORD.save() {
        Ok(_) => {
            log::info!("Discord config is updated.");
        },
        Err(error) => {
            log::error!("Discord unable to save config: {}", error);
        }
    }
}
