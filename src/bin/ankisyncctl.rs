use ankisyncd::user::*;
use std::{env, io};

fn usage() {
    println!("usage:  <command> [<args>]");
    println!();
    println!("Commands:");
    println!("  adduser <username> <newpasswd> - add a new user");
    println!("  deluser <username> - delete a user");
    println!("  lsuser             - list users");
    println!("  passwd <username> <usernewpasswd>  - change password of a user");
}

fn main() -> io::Result<()> {
    let mut v_args = vec![];
    for item in env::args() {
        v_args.push(item)
    }
    let arg_len = v_args.len();
    // create db
    create_auth_db().unwrap();

    match arg_len {
        x if x < 2 => {
            usage();
        }
        2 => {
            // lsusr
            let s = v_args.pop().unwrap();
            if s == "lsuser" {
                let usr_list = user_list().unwrap();
                match usr_list {
                    None => println!(""),
                    Some(usr) => {
                        let us = usr.join(" ");
                        println!("{}", us)
                    }
                }
            }
        }
        3 => {
            // delusr
            let s = v_args.get(1).unwrap();
            let usrname = v_args.last().unwrap();
            if s == "deluser" {
                del_user(&usrname).unwrap();
            } else {
            }
        }
        4 => {
            // addusr or passwd
            let s = v_args.get(1).unwrap();
            let args = &v_args[2..];
            if s == "passwd" {
                passwd(args).unwrap();
            } else if s == "adduser" {
                add_user(args).unwrap();
            }
        }
        _ => {
            usage();
        }
    }

    Ok(())
}
