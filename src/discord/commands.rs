use std::collections::HashSet;

use serenity::model::id::{UserId};
use serenity::model::channel::Message;
use serenity::prelude::{Context};
use serenity::framework::standard::{Args, CommandResult, CommandOptions, CheckResult, Check, HelpOptions, CommandGroup, help_commands, DispatchError, Reason};
use serenity::framework::standard::macros::{command, group, help};

use crate::{utils, config};
use crate::stats::{self, STATS};
use crate::constants::{Waifu, ADMIN_CHECK_FAIL, MSG_SET_WELCOME, MSG_REMOVE_WELCOME, MSG_REMOVE_SUB, MSG_ADD_SUB, MSG_UNKNOWN_SUB};

macro_rules! handle_msg_send {
    ($res:expr) => {
        match $res {
            Ok(_) => Ok(()),
            Err(serenity::Error::Http(error)) => match *error {
                serenity::prelude::HttpError::UnsuccessfulRequest(_) => {
                    STATS.increment(stats::DiscordMsgReject);
                    Err(error.into())
                },
                _ => {
                    STATS.increment(stats::DiscordMsgFail);
                    Err(error.into())
                },
            },
            Err(error) => {
                STATS.increment(stats::DiscordMsgReject);
                Err(error.into())
            },
        }
    }
}

static IS_ADMIN_CHECK: Check = Check {
    name: "Admin",
    function: is_admin,
    check_in_help: true,
    display_in_help: false
};

fn is_admin(ctx: &mut Context, message: &Message, _args: &mut Args, _options: &CommandOptions) -> CheckResult {
    let owner_id = config::DISCORD.with_read(|config| config.owner);
    if owner_id == message.author.id.0 {
        return CheckResult::Success;
    }

    if let Some(member) = message.member(&ctx.cache) {
        if let Ok(permissions) = member.permissions(&ctx.cache) {
            return permissions.administrator().into();
        }
    }

    CheckResult::Failure(Reason::Unknown)
}

#[group("general")]
#[commands(ping, dice, subscribe, set_waifu)]
#[description = "List of commands available for everyone"]
pub struct General;

#[command]
#[description = "Ping bot and get pong in response, if bot is alvie"]
#[max_args(0)]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    handle_msg_send!(msg.reply(ctx, "Pong!"))
}

#[command]
#[description = "Rolls DnD dice"]
#[example = "Example: 2d20+2, d4-1"]
#[min_args(1)]
fn dice(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let roll = args.rest();

    let res = match cute_dnd_dice::Roll::from_str(roll) {
        Ok(roll) => msg.reply(ctx, &format!("You roll {}", roll.roll())),
        Err(error) => msg.reply(ctx, &format!("Cannot parse your roll: {}. Try better", error)),
    };

    handle_msg_send!(res)
}

#[command]
#[description = "Performs subscribe/unsubscribe for notifications\n\
\n\
To unsubscribe, subscribe again.\n\
\n\
Available subscribtions:\n\
\n\
- Naze - Subscribes to notifications about Naze Boku no Sekai Rare mo Oboitenaika?\n\
- Bisokuzenshin - Subscribes to Azur Lane Slow Ahead 4koma TLs\n\
"]
#[num_args(1)]
fn subscribe(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let arg = match args.trimmed().quoted().current() {
        Some(arg) => arg,
        None => unreach!()
    };

    let ch_id = msg.channel_id.0;

    let text = if arg.eq_ignore_ascii_case("Naze") {
        config::DISCORD.with_write(|config| match config.channels.naze.take(&ch_id).is_some() {
            true => MSG_REMOVE_SUB,
            false => {
                config.channels.naze.insert(ch_id);
                MSG_ADD_SUB
            }
        })
    } else if arg.eq_ignore_ascii_case("Bisokuzenshin") {
        config::DISCORD.with_write(|config| match config.channels.bisokuzenshin.take(&ch_id).is_some() {
            true => MSG_REMOVE_SUB,
            false => {
                config.channels.bisokuzenshin.insert(ch_id);
                MSG_ADD_SUB
            }
        })
    } else {
        MSG_UNKNOWN_SUB
    };

    handle_msg_send!(msg.reply(ctx, text))
}

#[group("admin")]
#[commands(stats, debug, welcome)]
#[checks(is_admin)]
#[description = "List of commands available for administrators"]
pub struct Admin;

#[command]
#[description = "List bot's counters"]
#[max_args(0)]
fn stats(ctx: &mut Context, msg: &Message) -> CommandResult {
    let res = msg.channel_id.send_message(&ctx.http, |msg| {
            msg.embed(|embed| embed.title("Stats").color(serenity::utils::Colour::DARK_RED)
                                   .field("Discord", &STATS.discord, true)
                                   .field("Twitter", &STATS.twitter, true))
    });

    STATS.reset();

    handle_msg_send!(res)
}

#[command]
#[description = "Select your waifu"]
#[max_args(1)]
fn set_waifu(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let waifu = match args.current() {
        Some(waifu) => match Waifu::from_str(waifu) {
            Some(waifu) => waifu,
            None => {
                let res = msg.reply(&*ctx.http, "I don't know such girl.");
                return handle_msg_send!(res)
            }
        },
        None => {
            let res = msg.reply(&*ctx.http, "Who is your waifu among Rinne, Jeanne, Reiren and Hinemarill?");
            return handle_msg_send!(res)
        }
    };

    let mut member = match msg.member(&ctx.cache) {
        Some(member) => member,
        None => {
            let res = msg.reply(&*ctx.http, "This command is available in guild only");
            return handle_msg_send!(res)
        }
    };

    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            let res = msg.reply(&*ctx.http, "This command is available in guild only");
            return handle_msg_send!(res)
        }
    };

    let mut to_remove_roles = utils::four::Vec::<serenity::model::id::RoleId>::new();
    let mut waifu_role = serenity::model::id::RoleId(0);
    let waifu_str = waifu.as_str();

    for (id, role) in guild.read().roles.iter() {
        if !role.name.starts_with("Team") {
            continue;
        }

        if role.name.ends_with(waifu_str) {
            waifu_role = *id;
        } else {
            to_remove_roles.push(*id);
        }
    }

    if member.roles.contains(&waifu_role) {
        return handle_msg_send!(msg.reply(&*ctx.http, "Yes, I know that she is your waifu, you don't need to repeat"));
    }

    let _ = member.remove_roles(&*ctx.http, to_remove_roles.as_slice());
    let res = match member.add_role(&*ctx.http, waifu_role) {
        Ok(_) => msg.reply(&*ctx.http, format!("Set your waifu as {}", waifu_str)),
        Err(err) => {
            rogu::error!("Failed to set role. Error: {:?}", err);
            msg.reply(&*ctx.http, "Cannot set waifu :(")
        }
    };

    return handle_msg_send!(res)
}

#[command]
#[description = "Debug bot"]
fn debug(ctx: &mut Context, msg: &Message) -> CommandResult {
    let _ = msg.reply(&*ctx.http, format!("Raw msg: `{:?}`", msg.content.clone()));
    let res = msg.reply(&*ctx.http, format!("Clean msg: `{:?}`", msg.content_safe(&ctx.cache)));

    handle_msg_send!(res)
}

#[command]
#[description = "Sets welcome channel for new users"]
#[max_args(0)]
fn welcome(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel_id = msg.channel_id.0;

    let rsp = config::DISCORD.with_write(move |config| match config.channels.welcome == channel_id {
        true => {
            config.channels.welcome = 0;
            MSG_REMOVE_WELCOME
        },
        false => {
            config.channels.welcome = channel_id;
            MSG_SET_WELCOME
        },
    });

    handle_msg_send!(msg.reply(ctx, rsp))
}

#[help]
#[individual_command_tip = "Usage"]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(3)]
#[lacking_permissions = "strike"]
#[lacking_role = "strike"]
#[lacking_ownership = "strike"]
fn help(context: &mut Context, msg: &Message, args: Args, help_options: &'static HelpOptions, groups: &[&'static CommandGroup], owners: HashSet<UserId>) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

pub fn on_dispatch_error(ctx: &mut serenity::prelude::Context, message: &serenity::model::channel::Message, error: DispatchError) {
    let res = match error {
        DispatchError::Ratelimited(remaining) => {
            let minutes = remaining / 60;
            let seconds = remaining % 60;

            match minutes {
                0 => message.reply(ctx, &format!("Please wait just a bit more. Remains {} seconds", remaining)),
                _ => message.reply(ctx, &format!("You shouldn't do it so much. Wait {}:{} minutes", minutes, seconds)),
            }
        }
        DispatchError::NotEnoughArguments {min, given} => message.reply(ctx, &format!("Command needs at least {} arguments, {} were given", min, given)),
        DispatchError::TooManyArguments {max, given} => message.reply(ctx, &format!("Command needs no more than {} arguments, {} were given", max, given)),
        DispatchError::CheckFailed("Admin", _) => message.reply(ctx, ADMIN_CHECK_FAIL),
        _ => return,
    };

    match res {
        Ok(_) => (),
        Err(_) => {
            STATS.increment(stats::DiscordMsgReject);
        }
    }
}
