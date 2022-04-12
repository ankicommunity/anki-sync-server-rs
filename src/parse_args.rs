use crate::config::Config;
use crate::error::ApplicationError;
use crate::user::user_manage;
use clap::{crate_description, crate_name, crate_version, Arg, ArgMatches, Command};

/// construct a argument parser and parse args
pub fn parse_arguments() -> ArgMatches {
    Command::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file,ie -c ankisyncd.toml")
                .takes_value(true),
        )
        .arg(
            Arg::new("defaults")
                .short('d')
                .long("defaults")
                .help("Show the default configuration")
                .takes_value(false),
        )
        .subcommand(
            Command::new("user")
                .short_flag('U')
                .about("user management,interact with db CRUD actions")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("add")
                        .long("add")
                        .short('a')
                        .help("create user account, i.e.-a user password")
                        .value_names(&["username", "password"])
                        .takes_value(true)
                        .multiple_values(true)
                        .number_of_values(2),
                )
                .arg(
                    Arg::new("del")
                        .long("del")
                        .short('d')
                        .help("delete users,allow for multi-users, i.e.-d  user1 user2")
                        .value_name("username")
                        .takes_value(true)
                        .multiple_values(true)
                        .min_values(1),
                )
                .arg(
                    Arg::new("pass")
                        .long("pass")
                        .short('p')
                        .help("change user's password, i.e.-p user newpassword")
                        .value_names(&["username", "newpassword"])
                        .takes_value(true)
                        .multiple_values(true)
                        .number_of_values(2),
                )
                .arg(
                    Arg::new("list")
                        .help("list all usernames extracted from db ,i.e. -l")
                        .long("list")
                        .short('l'),
                ),
        )
        .get_matches()
}

/// Get config from path (if specified) or default value,
pub fn config_from_arguments(arg: &ArgMatches) -> Result<Config, ApplicationError> {
    if let Some(p) = arg.value_of("config") {
        return Config::from_file(p);
    }
    Ok(Config::default())
}

/// Manage user
pub fn manage_user(arg: &ArgMatches, auth_path: &str) -> bool {
    // TODO: better condition there
    if arg.subcommand_name().is_some() {
        if let Err(e) = user_manage(arg, auth_path) {
            panic!("Error managing users: {}", e);
        };
        return true;
    }
    false
}
