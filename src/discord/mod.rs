use crate::{config, constants};
use crate::stats::{self, STATS};
use crate::twitter;

use std::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

mod commands;

use commands::*;

static SELF_ID: AtomicU64 = AtomicU64::new(0);

lazy_static::lazy_static! {
    pub static ref HTTP: parking_lot::RwLock<Option<Arc<serenity::CacheAndHttp>>> = parking_lot::RwLock::new(None);
}

#[inline(always)]
fn get_reaction_server_emoji(id: u64, name: &str) -> serenity::model::channel::ReactionType {
    serenity::model::misc::EmojiIdentifier {
        id: id.into(),
        name: name.into(),
    }.into()
}

fn stat_serenity_error(error: &serenity::Error) {
    use core::ops::Deref;

    match error {
        serenity::Error::Http(ref error) => match error.deref() {
            serenity::prelude::HttpError::UnsuccessfulRequest(_) => {
                STATS.increment(stats::DiscordMsgReject);
            },
            _ => {
                STATS.increment(stats::DiscordMsgFail);
            }
        },
        _ => {
            STATS.increment(stats::DiscordMsgFail);
        }
    }
}

struct Handler {
    welcome_done: AtomicBool,
}

impl Handler {
    const fn new() -> Self {
        Self {
            welcome_done: AtomicBool::new(false),
        }
    }
}

impl serenity::client::EventHandler for Handler {
    fn ready(&self, ctx: serenity::prelude::Context, _bot_data: serenity::model::gateway::Ready) {
        STATS.increment(stats::DiscordConnected);

        if !self.welcome_done.compare_and_swap(false, true, Ordering::AcqRel) {
            let welcome_channel = config::DISCORD.with_read(|config| config.channels.welcome);

            if welcome_channel > 0 {
                let welcome_channel = serenity::model::id::ChannelId(welcome_channel);
                match welcome_channel.say(&ctx.http, constants::JEANNE_GREETING) {
                    Ok(_) => (),
                    Err(error) => {
                        rogu::error!("Unable to greet on discord. Error: {}", error);
                        stat_serenity_error(&error);
                    }
                }
            }
        }
    }

    fn resume(&self, _ctx: serenity::prelude::Context, _: serenity::model::event::ResumedEvent) {
        STATS.increment(stats::DiscordReConnected);
    }

    fn guild_member_addition(&self, ctx: serenity::prelude::Context, _: serenity::model::id::GuildId, user: serenity::model::guild::Member) {
        use serenity::model::misc::Mentionable;

        let welcome_channel = config::DISCORD.with_read(|config| config.channels.welcome);
        if welcome_channel > 0 {
            let welcome_channel = serenity::model::id::ChannelId(welcome_channel);

            let mention = {
                let user = user.user.read();

                if user.bot {
                    return;
                }

                user.id.mention()
            };

            match welcome_channel.say(&ctx.http, format_args!("@here Everyone, please welcome {}", mention)) {
                Ok(_) => (),
                Err(error) => stat_serenity_error(&error),
            }
        }

        STATS.increment(stats::DiscordNewMember);
    }

    fn guild_member_removal(&self, _: serenity::prelude::Context, _: serenity::model::id::GuildId, user: serenity::model::user::User, _: Option<serenity::model::guild::Member>) {
        if user.bot {
            return;
        }

        STATS.increment(stats::DiscordLossMember);
    }

    fn message(&self, ctx: serenity::prelude::Context, msg: serenity::model::prelude::Message) {
        if msg.author.bot {
            return;
        }

        let self_id = SELF_ID.load(Ordering::Acquire);

        if self_id == 0 || msg.author.id.0 == self_id {
            return;
        }

        if msg.mention_everyone {
            if let Err(error) = msg.react(&*ctx.http, get_reaction_server_emoji(constants::emoji::jeanne::hmph::ID, constants::emoji::jeanne::hmph::NAME)) {
                rogu::error!("Cannot react with hmph. Error={}", error);
                stat_serenity_error(&error);
            }
            return;
        }

        if msg.mentions.len() == 1 {
            if msg.mentions[0].id.0 == self_id {
                if let Err(error) = msg.react(&*ctx.http, get_reaction_server_emoji(constants::emoji::jeanne::smile::ID, constants::emoji::jeanne::smile::NAME)) {
                    rogu::error!("Cannot react with smile. Error={}", error);
                    stat_serenity_error(&error);
                }
            }
        }
    }
}

fn configure(config: &mut serenity::framework::standard::Configuration) -> &mut serenity::framework::standard::Configuration {
    config.prefix(config::CMD_PREFIX)
          .ignore_bots(true)
          .case_insensitivity(true)
          .allow_dm(true)
}

pub fn run() {
    let mut client = serenity::client::Client::new(config::DISCORD_TOKEN, Handler::new()).expect("To create client");

    client.with_framework(
        serenity::framework::StandardFramework::new().configure(configure)
                                                     .help(&HELP)
                                                     .on_dispatch_error(on_dispatch_error)
                                                     .group(&GENERAL_GROUP)
                                                     .group(&ADMIN_GROUP)
    );

    match client.cache_and_http.http.get_current_user() {
        Ok(info) => {
            SELF_ID.store(info.id.0, Ordering::Release);
        },
        Err(error) => {
            rogu::error!("Discord unable to get current user info: {}", error);
        }
    }

    if config::DISCORD.with_read(|config| config.owner) == 0 {
        match client.cache_and_http.http.get_current_application_info() {
            Ok(info) => {
                rogu::info!("Discord setting new owner id={}", info.owner.id.0);
                config::DISCORD.with_write(|config| config.owner = info.owner.id.0);
            },
            Err(error) => {
                rogu::error!("Discord unable to get application information: {}", error);
                STATS.increment(stats::DiscordNoAppInfo);
            }
        };
    }

    HTTP.write().replace(client.cache_and_http.clone());

    while let Some((tweet_id, user_name, tweet_type)) = twitter::BUFFERED_TWEETS.dequeue() {
        twitter::redirect_tweet(&client.cache_and_http.http, tweet_id, user_name, tweet_type);
    }

    loop {
        rogu::info!("Discord: start");
        match client.start() {
            Ok(_) => {
                STATS.increment(stats::DiscordShutdown);
                break;
            }
            Err(error) => {
                STATS.increment(stats::DiscordFailure);
                rogu::warn!("Discord stopped with error: {}", error);
            }
        }
    }

    HTTP.write().take();
}
