pub const DISCORD_TOKEN: &str = env!("JEANNE_DISCORD_TOKEN");
pub const CMD_PREFIX: &str = "~";
pub const NAME: &'static str = "discord.toml";

pub fn init() {
    let _ = cute_log::init();
}
