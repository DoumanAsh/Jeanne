use crate::config;
use crate::stats::{self, STATS};

mod commands;

use commands::*;

struct Handler;

impl serenity::client::EventHandler for Handler {
    fn ready(&self, _ctx: serenity::prelude::Context, _bot_data: serenity::model::gateway::Ready) {
        STATS.increment(stats::DiscordConnected);
    }

    fn resume(&self, _ctx: serenity::prelude::Context, _: serenity::model::event::ResumedEvent) {
        STATS.increment(stats::DiscordReConnected);
    }

    fn guild_member_addition(&self, _: serenity::prelude::Context, _: serenity::model::id::GuildId, user: serenity::model::guild::Member) {
        let _user_id = {
            let user = user.user.read();

            if user.bot {
                return;
            }

            user.id.0 as i64
        };

        STATS.increment(stats::DiscordNewMember);
    }

    fn guild_member_removal(&self, _: serenity::prelude::Context, _: serenity::model::id::GuildId, user: serenity::model::user::User, _: Option<serenity::model::guild::Member>) {
        if user.bot {
            return;
        }

        STATS.increment(stats::DiscordLossMember);
    }

}

fn configure(config: &mut serenity::framework::standard::Configuration) -> &mut serenity::framework::standard::Configuration {
    config.prefix(config::CMD_PREFIX)
          .ignore_bots(true)
          .case_insensitivity(true)
          .allow_dm(true)
}

pub fn run() {
    let mut client = serenity::client::Client::new(config::DISCORD_TOKEN, Handler).expect("To create client");

    client.with_framework(
        serenity::framework::StandardFramework::new().configure(configure)
                                                     .help(&HELP)
                                                     .on_dispatch_error(on_dispatch_error)
                                                     .group(&GENERAL_GROUP)
                                                     .group(&ADMIN_GROUP)
    );


    if config::DISCORD.with_read(|config| config.owner) == 0 {
        match client.cache_and_http.http.get_current_application_info() {
            Ok(info) => {
                log::info!("Discord setting new owner id={}", info.owner.id.0);
                config::DISCORD.with_write(|config| config.owner = info.owner.id.0);
            },
            Err(error) => {
                log::error!("Discord unable to get application information: {}", error);
                STATS.increment(stats::DiscordNoAppInfo);
            }
        };
    }

    loop {
        log::info!("Discord: start");
        match client.start() {
            Ok(_) => {
                STATS.increment(stats::DiscordShutdown);
                break;
            }
            Err(error) => {
                STATS.increment(stats::DiscordFailure);
                log::warn!("Discord stopped with error: {}", error);
            }
        }
    }
}
