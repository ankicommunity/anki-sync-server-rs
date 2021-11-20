use clap::{App, Arg, ArgMatches};
use config::{Environment, File};
use std::collections::HashMap;
/// construct a argument parser
pub fn parse() -> ArgMatches {
    App::new("ankisyncd")
        .version("0.1.2")
        .author("a member of ankicommunity")
        .about("a person anki sync server")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .about("Sets a custom config file,ie -c ankisyncd.toml")
                .default_value("Settings.toml")
                .takes_value(true),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .takes_value(true)
                .about("Sets the level of verbosity"),
        )
        .subcommand(
            App::new("adduser")
                .about("create account,insert account to database")
                .arg(Arg::new("username").about("ie qingqing").required(true))
                .arg(Arg::new("password").about("ie 123456").required(true)),
        )
        .subcommand(
            App::new("deluser")
                .about("delete user(s) from database")
                .arg("<users>... 'A sequence of users, i.e. user1 user2'"), // .arg(Arg::new("username").about("ie user1 user2").required(true))
        )
        .subcommand(App::new("lsuser").about("show existing users"))
        .subcommand(
            App::new("passwd")
                .about("change user password")
                .arg(Arg::new("username").about("ie qingqing").required(true))
                .arg(Arg::new("newpassword").about("ie 123456").required(true)),
        )
        .get_matches()
}
pub fn addr() -> String {
    let envs = env_variables();
    let h = envs.get("host").unwrap();
    let p = envs.get("port").unwrap();
    format!("{}:{}", h, p)
}

/// return sync env vars in hashmap
pub fn env_variables() -> HashMap<String, String> {
    let mut settings = config::Config::default();
    settings
        // Add in `./Settings.toml`
        .merge(File::with_name("Settings"))
        .unwrap()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(Environment::with_prefix("host"))
        .unwrap()
        .merge(Environment::with_prefix("port"))
        .unwrap()
        .merge(Environment::with_prefix("data"))
        .unwrap()
        .merge(Environment::with_prefix("base"))
        .unwrap()
        .merge(Environment::with_prefix("auth"))
        .unwrap()
        .merge(Environment::with_prefix("session"))
        .unwrap()
        .merge(Environment::with_prefix("ssl"))
        .unwrap()
        .merge(Environment::with_prefix("cert"))
        .unwrap()
        .merge(Environment::with_prefix("key"))
        .unwrap()
        .merge(Environment::with_prefix("user"))
        .unwrap();

    settings.try_into::<HashMap<String, String>>().unwrap()
}

// #[test]
// fn test_env_vars() {
//     println!("{:?}", env_variables())
// }
