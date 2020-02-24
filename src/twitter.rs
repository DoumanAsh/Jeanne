use crate::{config, discord, constants};
use crate::stats::{self, STATS};
use crate::utils::mpmc::Q64;

use std::borrow::Cow;

pub const TWITTER_CONSUMER_KEY: &str = env!("JEANNE_TWITTER_CONSUMER_KEY");
pub const TWITTER_CONSUMER_SECRET: &str = env!("JEANNE_TWITTER_CONSUMER_SECRET");
pub const TWITTER_ACCESS_KEY: &str = env!("JEANNE_ACCESS_CONSUMER_KEY");
pub const TWITTER_ACCESS_SECRET: &str = env!("JEANNE_ACCESS_CONSUMER_SECRET");

const TOKEN: egg_mode::Token = egg_mode::Token::Access {
    consumer: egg_mode::KeyPair {
        key: Cow::Borrowed(TWITTER_CONSUMER_KEY),
        secret: Cow::Borrowed(TWITTER_CONSUMER_SECRET),
    },
    access: egg_mode::KeyPair {
        key: Cow::Borrowed(TWITTER_ACCESS_KEY),
        secret: Cow::Borrowed(TWITTER_ACCESS_SECRET),
    }
};

//Stores cached tweet data,
//we most likely do not need such big capacity
//but just in case.
pub static BUFFERED_TWEETS: Q64<(u64, String, TweetType)> = Q64::new();

fn create_twitter_stream() -> egg_mode::stream::TwitterStream {
    egg_mode::stream::filter().filter_level(egg_mode::stream::FilterLevel::None)
                              .track(&["#びそくぜんしんっ", "#なぜ僕", "#なぜ僕の世界を誰も覚えていないのか"])
                              .start(&TOKEN)
}

pub enum TweetType {
    NazeBoku,
    Bisokuzenshin,
}

fn send_tweet(http: &serenity::http::client::Http, id: u64, name: &str, ch_id: u64) {
    STATS.increment(stats::TwitterRetweet);
    match serenity::model::id::ChannelId(ch_id).say(http, format_args!("https://twitter.com/{}/status/{}", name, id)) {
        Ok(_) => (),
        Err(serenity::Error::Http(error)) => match *error {
            serenity::prelude::HttpError::UnsuccessfulRequest(_) => {
                STATS.increment(stats::DiscordMsgReject);
            },
            error => {
                rogu::warn!("Twitter redirect failed with error: {}", error);
                STATS.increment(stats::DiscordMsgFail);
            },
        },
        Err(error) => {
            rogu::warn!("Twitter redirect failed with error: {}", error);
            STATS.increment(stats::DiscordMsgReject);
        },
    }
}

pub fn redirect_tweet(http: &serenity::http::client::Http, id: u64, name: String, typ: TweetType) {
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

async fn retweet(id: u64) {
    match egg_mode::tweet::retweet(id, &TOKEN).await {
        Ok(_) => (),
        Err(error) => {
            rogu::warn!("Unable to retweet id={}. Error: {}", id, error);
        }
    }
}

fn get_jeanne_phrase() -> &'static str {
    use rand::distributions::{Distribution, Uniform};

    let distribution = Uniform::from(0..constants::JEANNE_TALK.len());
    let mut rng = rand::thread_rng();
    constants::JEANNE_TALK[distribution.sample(&mut rng)]
}

async fn talk() {
    let mut interval = async_timer::Interval::platform_new(core::time::Duration::from_secs(86400));
    loop {
        interval.as_mut().await;
        match egg_mode::tweet::DraftTweet::new(get_jeanne_phrase()).send(&TOKEN).await {
            Ok(_) => (),
            Err(error) => {
                rogu::warn!("Unable to send phrase. Error: {}", error);
            }
        }
    }
}

#[tokio::main]
pub async fn worker() {
    use futures_util::stream::StreamExt;

    tokio::spawn(talk());

    loop {
        rogu::info!("Twitter stream starting...");
        STATS.increment(stats::TwitterStartStream);

        let mut stream = create_twitter_stream();

        'msg: while let Some(Ok(msg)) = stream.next().await {
            match msg {
                egg_mode::stream::StreamMessage::Tweet(tweet) => if tweet.retweeted_status.is_none() {
                    let name = match tweet.user {
                        Some(user) => user.screen_name,
                        None => continue,
                    };

                    rogu::debug!("Incoming tweet from user={}, id={}", name, tweet.id);

                    for hash_tag in tweet.entities.hashtags {
                        if hash_tag.text == "びそくぜんしんっ" {
                            place_tweet(tweet.id, name, TweetType::Bisokuzenshin);
                            continue 'msg;
                        } else if hash_tag.text.starts_with("なぜ僕") {
                            place_tweet(tweet.id, name, TweetType::NazeBoku);
                            tokio::spawn(retweet(tweet.id));
                            continue 'msg;
                        }
                    }

                    //tweet.entities.hashtags doesn't contain hashtags for long tweets
                    if tweet.text.contains("びそくぜんしんっ") {
                        place_tweet(tweet.id, name, TweetType::Bisokuzenshin);
                        continue;
                    } else if tweet.text.contains("なぜ僕") {
                        place_tweet(tweet.id, name, TweetType::NazeBoku);
                        tokio::spawn(retweet(tweet.id));
                        continue;
                    }
                },
                egg_mode::stream::StreamMessage::Disconnect(code, error) => {
                    rogu::warn!("Twitter disconnected. Code={}, Error={}", code, error);
                    break;
                }
                _ => (),
            }
        }
    }
}
