use clap::{App, Arg, ArgMatches};

pub mod conf {
    use std::fs;
    use std::path::Path;

    pub fn write_conf(p: &Path) {
        let content = r#"
[address]
host="0.0.0.0"
port = "27701"
#set real data path with ENV ANKISYNCD_ROOT,if not exist,
# use current executable path ,only set filename
[path]
data_root = "collections"
auth_db_path = "auth.db"
session_db_path = "session.db"
        
# following fields will be added 
#into auth.db if not empty,and two fields must not be empty
[account]
username=""
userpassword=""
        
# embeded encrypted http /https credential if in Intranet
# true to enable ssl or false
[localcert]
ssl_enable="false"
cert_file=""
key_file=""
"#;
        if !p.exists() {
            fs::write(&p, content).unwrap();
        }
    }
}
/// construct a argument parser
pub fn parse() -> ArgMatches {
    App::new("ankisyncd")
        .version("v0.1.2")
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
