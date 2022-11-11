use crate::config::Config;
use crate::error::ApplicationError;
use crate::user::user_manage;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[clap( version,about, long_about = None)]
pub struct Arg {
    ///Sets a custom config file,ie -c ankisyncd.toml
    #[clap(short, long, value_parser, value_name("file"))]
    pub(crate) config: Option<PathBuf>,
    /// Show the default configuration
    #[clap(short, long, action)]
    pub(crate) default: bool,
    #[command(subcommand)]
    pub(crate) cmd: Option<UserCommand>,
}
#[derive(clap::Subcommand, Debug)]
pub enum UserCommand {
    /// user management,interact with db CRUD actions
    User {
        /// create user account, i.e.ankisyncd user -a username password
        #[clap(short, long, value_parser,number_of_values(2),value_names(&["username", "password"]))]
        add: Option<Vec<String>>,
        /// delete users,allow for multi-users, i.e.ankisyncd user -d  username1 username2
        #[clap(short, long, value_parser, value_name("username"))]
        del: Option<Vec<String>>,
        /// change user's password, i.e.ankisyncd user -p username newpassword
        #[clap(short, long, value_parser,number_of_values(2),value_names(&["username", "password"]))]
        pass: Option<Vec<String>>,
        /// list all usernames extracted from db ,i.e.ankisyncd user  -l
        #[clap(short, long, action)]
        list: bool,
    },
}

/// Get config from path (if specified) or default value,
pub fn config_from_arguments(arg: &Arg) -> Result<Config, ApplicationError> {
    if let Some(p) = arg.config.as_ref() {
        return Config::from_file(p);
    }
    Ok(Config::default())
}

/// Manage user
pub fn manage_user(cmd: &UserCommand, auth_path: &str) {
    if let Err(e) = user_manage(cmd, auth_path) {
        panic!("Error managing users: {}", e);
    };
}
