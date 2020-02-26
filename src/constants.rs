use core::time::Duration;

pub const ADMIN_CHECK_FAIL: &str = "You're unathorized to access the command.";

pub const MSG_SET_WELCOME: &str = "I've set this channel as welcoming one.";
pub const MSG_REMOVE_WELCOME: &str = "This channel is no longer welcoming one.";
pub const MSG_REMOVE_SUB: &str = "Removed subscribtion.";
pub const MSG_ADD_SUB: &str = "Added subscribtion.";
pub const MSG_UNKNOWN_SUB: &str = "Unknown type of subscribtion, please check command help.";

pub const CONFIG_UPDATE_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub const JEANNE_GREETING: &str = "諸君、おはようございます";

pub const JEANNE_TALK: [&str; 5] = [
    "最近花琳からジャンニャの名前聞いた\n誰それは？部下中にはそんな人があったかな？",
    "カイはいつもリンネことを甘やかす\n子供か？\nええ、羨ましいですか？私！？\n花琳、何を言っている！？\n リンネがただ狡い！",
    "花琳は気になる?\n特別の関係ですか？\nまぁ長い付き合いからね...\nえええ、親子みたい！？",
    "マッサージはすき?\nまぁずっと鎧を着ているから夜で肩はいつも痛み\nだからマッサージは歓迎",
    "まったく、カイはいつも無茶をしてる\nなぜ私を頼ってならない？\n特別扱い？べー別にそんなつもりはない...\n指揮官にとしてそれは普通だ\n笑えない、花琳！",
];

pub mod emoji {
    pub mod jeanne {
        pub mod hmph {
            pub const ID: u64 = 613991053065977876;
            pub const NAME: &str = "JeanneHmph";
        }

        pub mod smile {
            pub const ID: u64 = 613992618208133120;
            pub const NAME: &str = "JeanneSmile";
        }
    }
}

pub fn get_jeanne_phrase() -> &'static str {
    use rand::distributions::{Distribution, Uniform};

    let distribution = Uniform::from(0..JEANNE_TALK.len());
    let mut rng = rand::thread_rng();
    JEANNE_TALK[distribution.sample(&mut rng)]
}

