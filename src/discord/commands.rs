use std::collections::HashSet;
use std::collections::hash_map;

use serenity::model::id::{UserId};
use serenity::model::channel::Message;
use serenity::prelude::{Context};
use serenity::framework::standard::{Args, CommandResult, CommandOptions, CheckResult, Check, HelpOptions, CommandGroup, help_commands, DispatchError, Reason};
use serenity::framework::standard::macros::{command, group, help};

use crate::config;
use crate::stats::{self, STATS};
use crate::constants::{ADMIN_CHECK_FAIL, MSG_SET_WELCOME, MSG_REMOVE_WELCOME};

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

group!({
    name: "general",
    options: {
        description: "List of commands available for everyone"
    },
    commands: [ping, dice],
});

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

group!({
    name: "admin",
    options: {
        description: "List of commands for administrator",
        checks: [is_admin]
    },
    commands: [stats, welcome],
});

#[command]
#[description = "List bot's counters"]
#[max_args(0)]
fn stats(ctx: &mut Context, msg: &Message) -> CommandResult {
    let res = msg.channel_id.send_message(&ctx.http, |msg| {
            msg.embed(|embed| embed.title("Stats").color(serenity::utils::Colour::DARK_RED)
                                   .field("Discord", &STATS.discord, true))
    });

    STATS.reset();

    handle_msg_send!(res)
}

#[command]
#[description = "Sets welcome channel for new users"]
#[max_args(0)]
fn welcome(ctx: &mut Context, msg: &Message) -> CommandResult {
    let guild_id = match msg.guild_id {
        Some(guild_id) => guild_id,
        None => return Ok(()),
    };
    let channel_id = msg.channel_id.0;

    let rsp = config::DISCORD.with_write(move |config| match config.guilds.entry(guild_id.0) {
        hash_map::Entry::Occupied(mut occupied) => match occupied.get().channels.welcome == channel_id {
            true => {
                occupied.remove();
                MSG_REMOVE_WELCOME
            },
            false => {
                occupied.get_mut().channels.welcome = channel_id;
                MSG_SET_WELCOME
            }
        },
        hash_map::Entry::Vacant(vacant) => {
            let mut guild = config::discord::GuildInfo::default();
            guild.channels.welcome = channel_id;

            vacant.insert(guild);
            MSG_SET_WELCOME
        }
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
