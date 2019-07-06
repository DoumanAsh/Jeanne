use core::sync::atomic;
use core::fmt;
use core::marker::PhantomData;
use core::mem;

type Integer = atomic::AtomicUsize;

const fn default_integer() -> Integer {
    Integer::new(0)
}

pub trait CounterType {
    fn get_ref(stats: &Stats) -> &Integer;
}

macro_rules! impl_counter {
    ($($name:ident: $($path:tt).+;)+) => {
        $(
            pub struct $name;
            impl CounterType for $name {
                fn get_ref(stats: &Stats) -> &Integer {
                    &stats.$($path).+
                }
            }
        )+

            impl Stats {
                pub fn reset(&self) {
                    $(
                        $name::get_ref(self).store(0, atomic::Ordering::Release);
                     )+
                }
            }
    }
}

impl_counter!(
    DiscordConnected: discord.connected;
    DiscordReConnected: discord.re_connected;
    DiscordBrokenConfigUpdate: discord.broken_config_update;
    DiscordNoAppInfo: discord.no_app_info;
    DiscordMsgReject: discord.msg_reject;
    DiscordMsgFail: discord.msg_fail;
    DiscordShutdown: discord.shutdown;
    DiscordFailure: discord.failure;
    DiscordCmdNum: discord.cmd_count;
    DiscordNewMember: discord.new_member;
    DiscordLossMember: discord.loss_member;
    TwitterStartStream: twitter.start_stream;
    TwitterRetweet: twitter.retweet;
    TwitterUnfilteredTweet: twitter.unfiltered_tweet;
);

#[derive(Debug)]
pub struct Twitter {
    ///Number of times, twitter's stream has been started
    pub start_stream: Integer,
    ///Number of times redirected tweet.
    pub retweet: Integer,
    ///Number of times when incoming tweet was discarded due unmatching hash tags
    pub unfiltered_tweet: Integer,
}

#[derive(Debug)]
pub struct Discord {
    ///Discord has been connected.
    pub connected: Integer,
    ///Discord has been re-connected.
    pub re_connected: Integer,
    ///Unable to update configuration file.
    pub broken_config_update: Integer,
    ///Failed to retrieve application info.
    pub no_app_info: Integer,
    ///Message is rejected by Discord
    pub msg_reject: Integer,
    ///Failed to send message
    pub msg_fail: Integer,
    ///Serenity is gracefully shut down
    pub shutdown: Integer,
    ///Serenity aborted with error
    pub failure: Integer,
    ///Number of commands
    pub cmd_count: Integer,
    ///Number of new members
    pub new_member: Integer,
    ///Number of removed members
    pub loss_member: Integer,
}

impl fmt::Display for Twitter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "start_stream:     **{}**\n", self.start_stream.load(atomic::Ordering::Acquire))?;
        write!(f, "retweet:          **{}**\n", self.retweet.load(atomic::Ordering::Acquire))?;
        write!(f, "unfiltered_tweet: **{}**\n", self.unfiltered_tweet.load(atomic::Ordering::Acquire))?;

        Ok(())
    }
}

impl fmt::Display for Discord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "connected:            **{}**\n", self.connected.load(atomic::Ordering::Acquire))?;
        write!(f, "re_connected:         **{}**\n", self.re_connected.load(atomic::Ordering::Acquire))?;
        write!(f, "broken_config_update: **{}**\n", self.broken_config_update.load(atomic::Ordering::Acquire))?;
        write!(f, "no_app_info:          **{}**\n", self.no_app_info.load(atomic::Ordering::Acquire))?;
        write!(f, "msg_reject:           **{}**\n", self.msg_reject.load(atomic::Ordering::Acquire))?;
        write!(f, "msg_fail:             **{}**\n", self.msg_fail.load(atomic::Ordering::Acquire))?;
        write!(f, "shutdown:             **{}**\n", self.shutdown.load(atomic::Ordering::Acquire))?;
        write!(f, "failure:              **{}**\n", self.failure.load(atomic::Ordering::Acquire))?;
        write!(f, "cmd_count:            **{}**\n", self.cmd_count.load(atomic::Ordering::Acquire))?;
        write!(f, "new_member:           **{}**\n", self.new_member.load(atomic::Ordering::Acquire))?;
        write!(f, "loss_member:          **{}**\n", self.loss_member.load(atomic::Ordering::Acquire))?;

        Ok(())
    }
}

pub struct Stats {
    pub discord: Discord,
    pub twitter: Twitter,
}

impl Stats {
    const fn new() -> Self {
        Stats {
            discord: Discord {
                connected: default_integer(),
                re_connected: default_integer(),
                broken_config_update: default_integer(),
                no_app_info: default_integer(),
                msg_reject: default_integer(),
                msg_fail: default_integer(),
                shutdown: default_integer(),
                failure: default_integer(),
                cmd_count: default_integer(),
                new_member: default_integer(),
                loss_member: default_integer(),
            },
            twitter: Twitter {
                start_stream: default_integer(),
                retweet: default_integer(),
                unfiltered_tweet: default_integer(),
            }
        }
    }

    ///Increments value of counter, and returns its old value.
    pub fn increment<C: CounterType>(&self, _: C) -> StatIncrement<C> {
        StatIncrement {
            counter: PhantomData
        }
    }
}

pub struct StatIncrement<C: CounterType> {
    counter: PhantomData<C>
}

impl<C: CounterType> StatIncrement<C> {
    #[allow(unused)]
    pub fn forget(self) {
        mem::forget(self)
    }
}

impl<C: CounterType> Drop for StatIncrement<C> {
    fn drop(&mut self) {
        C::get_ref(&STATS).fetch_add(1, atomic::Ordering::AcqRel);
    }
}

pub static STATS: Stats = Stats::new();
