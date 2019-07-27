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
pub static BUFFERED_TWEETS: Q64<(u64, String, TweetType)> = Q64::new();

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

fn send_tweet(http: &serenity::http::raw::Http, id: u64, name: &str, ch_id: u64) {
    STATS.increment(stats::TwitterRetweet);
    match serenity::model::id::ChannelId(ch_id).say(http, format_args!("https://twitter.com/{}/status/{}", name, id)) {
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

pub fn redirect_tweet(http: &serenity::http::raw::Http, id: u64, name: String, typ: TweetType) {
    match typ {
        TweetType::NazeBoku => {
            config::DISCORD.with_read(move |config| for ch in config.channels.naze.iter() {
                send_tweet(&*http, id, &name, *ch);
            })
        },
        TweetType::Bisokuzenshin => {
            config::DISCORD.with_read(move |config| for ch in config.channels.bisokuzenshin.iter() {
                send_tweet(&*http, id, &name, *ch);
            })
        },
    }
}

fn place_tweet(id: u64, name: String, typ: TweetType) {
    let http = match discord::HTTP.read().as_ref() {
        Some(cache) => cache.http.clone(),
        None => {
            //Cache it for when discord re-connects
            let _ = BUFFERED_TWEETS.enqueue((id, name, typ));
            return;
        },
    };

    redirect_tweet(&*http, id, name, typ);
}

pub fn worker() {
    use tokio_global::futures::Stream;
    let _tokio = tokio_global::single::init();

    loop {
        log::info!("Twitter stream starting...");
        STATS.increment(stats::TwitterStartStream);

        let mut stream = create_twitter_stream();

        while let Ok((Some(msg), rem_stream)) = stream.into_future().finish() {
            stream = rem_stream;

            match msg {
                egg_mode::stream::StreamMessage::Tweet(tweet) => if tweet.retweeted_status.is_none() {
                    let name = match tweet.user {
                        Some(user) => user.screen_name,
                        None => continue,
                    };

                    log::debug!("Incoming tweet from user={}, id={}", name, tweet.id);

                    //Doesn't contain hashtags for long tweets
                    //tweet.entities.hashtags
                    if tweet.text.contains("びそくぜんしんっ") {
                        place_tweet(tweet.id, name, TweetType::Bisokuzenshin);
                        break;
                    } else if tweet.text.contains("なぜ僕") {
                        place_tweet(tweet.id, name, TweetType::NazeBoku);
                        break;
                    }
                },
                _ => (),
            }
        }
    }
}
