use core::time::Duration;

pub const ADMIN_CHECK_FAIL: &str = "You're unathorized to access the command.";

pub const MSG_SET_WELCOME: &str = "I've set this channel as welcoming one.";
pub const MSG_REMOVE_WELCOME: &str = "This channel is no longer welcoming one.";

pub const CONFIG_UPDATE_INTERVAL: Duration = Duration::from_secs(15 * 60);
