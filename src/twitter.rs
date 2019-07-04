use tokio_global::AutoRuntime;

use crate::{config, discord};
use crate::stats::{self, STATS};
use crate::utils::mpmc::Q64;

pub const TWITTER_CONSUMER_KEY: &str = env!("JEANNE_TWITTER_CONSUMER_KEY");
pub const TWITTER_CONSUMER_SECRET: &str = env!("JEANNE_TWITTER_CONSUMER_SECRET");
pub const TWITTER_ACCESS_KEY: &str = env!("JEANNE_ACCESS_CONSUMER_KEY");
pub const TWITTER_ACCESS_SECRET: &str = env!("JEANNE_ACCESS_CONSUMER_SECRET");

//Stores cached tweet data,
//we most likely do not need such big capacity
//but just in case.
pub static BUFFERED_TWEETS: Q64<(u64, TweetType)> = Q64::new();

fn create_twitter_stream() -> egg_mode::stream::TwitterStream {
    let token = egg_mode::Token::Access {
        consumer: egg_mode::KeyPair::new(TWITTER_CONSUMER_KEY, TWITTER_CONSUMER_SECRET),
        access: egg_mode::KeyPair::new(TWITTER_ACCESS_KEY, TWITTER_ACCESS_SECRET),
    };

    egg_mode::stream::filter().filter_level(egg_mode::stream::FilterLevel::None)
                              .track(&["#びそくぜんしんっ", "#なぜ僕の世界を誰も覚えていないのか"])
                              .start(&token)
}

pub enum TweetType {
    NazeBoku,
    Bisokuzenshin,
}

fn send_tweet(http: &serenity::http::raw::Http, id: u64, ch_id: u64) {
    match serenity::model::id::ChannelId(ch_id).say(http, format_args!("https://twitter.com/aksysgames/status/{}", id)) {
        Ok(_) => (),
        Err(serenity::Error::Http(error)) => match *error {
            serenity::prelude::HttpError::UnsuccessfulRequest(_) => {
                STATS.increment(stats::DiscordMsgReject);
            },
            error => {
                log::warn!("Twitter redirect failed with error: {}", error);
                STATS.increment(stats::DiscordMsgFail);
            },
        },
        Err(error) => {
            log::warn!("Twitter redirect failed with error: {}", error);
            STATS.increment(stats::DiscordMsgReject);
        },
    }
}

pub fn redirect_tweet(http: &serenity::http::raw::Http, id: u64, typ: TweetType) {
    match typ {
        TweetType::NazeBoku => {
            config::DISCORD.with_read(|config| for ch in config.channels.naze.iter() {
                send_tweet(&*http, id, *ch);
            })
        },
        TweetType::Bisokuzenshin => {
            config::DISCORD.with_read(|config| for ch in config.channels.bisokuzenshin.iter() {
                send_tweet(&*http, id, *ch);
            })
        },
    }
}

fn place_tweet(id: u64, typ: TweetType) {
    let http = match discord::HTTP.read().as_ref() {
        Some(cache) => cache.http.clone(),
        None => {
            //Cache it for when discord re-connects
            let _ = BUFFERED_TWEETS.enqueue((id, typ));
            return;
        },
    };

    redirect_tweet(&*http, id, typ);
}

pub fn worker() {
    use tokio_global::futures::Stream;
    let _tokio = tokio_global::single::init();

    loop {
        log::info!("Twitter stream starting...");

        let mut stream = create_twitter_stream();

        while let Ok((Some(msg), rem_stream)) = stream.into_future().finish() {
            stream = rem_stream;

            match msg {
                egg_mode::stream::StreamMessage::Tweet(tweet) => if !tweet.retweeted.unwrap_or(false) {
                    for hash_tag in tweet.entities.hashtags {
                        if hash_tag.text.contains("びそくぜんしんっ") {
                            place_tweet(tweet.id, TweetType::Bisokuzenshin);
                        } else if hash_tag.text.contains("なぜ僕") {
                            place_tweet(tweet.id, TweetType::NazeBoku);
                        }
                    }
                },
                _ => (),
            }
        }
    }
}
