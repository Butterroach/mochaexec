/*
mochaexec: sudo if it was actually good. for responsible adults only.
Copyright (C) 2025-2026 Butterroach

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

mod branding;

use nix::unistd::{Uid, User, execvp, setuid};
use owo_colors::OwoColorize;
use pam_client::conv_mock::Conversation;
use pam_client::{Context, Flag};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::{thread, time::Duration};
use strfmt::strfmt;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

#[derive(Deserialize, Debug)]
struct Config {
    prompt: String,
    prompt_start_color: [u8; 3],
    prompt_end_color: [u8; 3],
}

fn gradient_text(text: &str, start_rgb: (u8, u8, u8), end_rgb: (u8, u8, u8)) -> String {
    let len = text.chars().count().max(1) as f32;
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            let ratio = i as f32 / (len - 1.0);
            let r = start_rgb.0 as f32 + (end_rgb.0 as f32 - start_rgb.0 as f32) * ratio;
            let g = start_rgb.1 as f32 + (end_rgb.1 as f32 - start_rgb.1 as f32) * ratio;
            let b = start_rgb.2 as f32 + (end_rgb.2 as f32 - start_rgb.2 as f32) * ratio;
            c.truecolor(r as u8, g as u8, b as u8).to_string()
        })
        .collect::<String>()
}

fn prompt_password(prompt: &str) -> io::Result<String> {
    let tty = File::open("/dev/tty")?;
    let tty_reader = BufReader::new(tty);
    let mut stdout = io::stdout().into_raw_mode()?;

    print!("{}", prompt);
    stdout.flush()?;

    let mut password = String::new();

    for key in tty_reader.keys() {
        let key = key.map_err(|e| io::Error::new(io::ErrorKind::Other, e));
        match key {
            Ok(termion::event::Key::Char('\n')) | Ok(termion::event::Key::Char('\r')) => break,
            Ok(termion::event::Key::Backspace) => {
                if password.pop().is_some() {
                    print!("\x08 \x08");
                    stdout.flush()?;
                }
            }
            Ok(termion::event::Key::Char(c)) => {
                password.push(c);
                print!("*");
                stdout.flush()?;
            }
            Err(e) => return Err(e),
            _ => {}
        }
    }

    println!();
    Ok(password)
}

fn authenticate(username: &str, config: &Config) -> Result<(), pam_client::Error> {
    let retries = 5;
    let cooldown = Duration::from_secs(5);
    let mut last_err = None;

    let mut vars = HashMap::new();
    vars.insert("username".to_string(), username.to_string());
    vars.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    vars.insert(
        "shorthand".to_string(),
        branding::PROJECT_NAME_SHORTHAND.to_string(),
    );
    vars.insert("name".to_string(), branding::PROJECT_NAME.to_string());

    for attempt in 1..=retries {
        let password = prompt_password(&gradient_text(
            &*strfmt(&*config.prompt, &vars).unwrap(),
            <(u8, u8, u8)>::from(config.prompt_start_color),
            <(u8, u8, u8)>::from(config.prompt_end_color),
        ))
        .unwrap(); // i don't care if this panics i give up trying to use the stupid question mark
        let conv = Conversation::with_credentials(username, password);
        let mut context = Context::new("login", Some(username), conv)?;

        match context.authenticate(Flag::NONE) {
            Ok(_) => {
                print!("\x1b[2K\r");
                io::stdout().flush().unwrap();
                return Ok(());
            }
            Err(e) => last_err = Some(e),
        }

        print!("\x1b[2K\r");
        io::stdout().flush().unwrap();

        if attempt < retries {
            thread::sleep(cooldown);
            println!("wrong lmao")
        }
    }

    Err(last_err.expect("no error??? after failed auth????"))
}

fn read_allowed_users(path: &str) -> Result<Vec<String>, io::Error> {
    let content = fs::read_to_string(path)?;
    let users = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();
    Ok(users)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config: Config;
    match fs::read_to_string("/etc/mochaexec.d/config.toml") {
        Ok(content) => {
            config = toml::from_str(&content)?;
        }
        Err(_e) => {
            println!("no config file!!!! that means your setup is likely BROKEN. GO AWAY!!!");
            std::process::exit(0x1);
        }
    }

    match read_allowed_users("/etc/mochaexec.d/responsible_adults") {
        Ok(users) => {
            let username = &User::from_uid(nix::unistd::getuid())
                .expect("getting uid failed for some reason")
                .expect("current user is a ghost and their uid does not exist")
                .name;
            if users.contains(username) {
                match authenticate(username, &config) {
                    Ok(_) => {}
                    Err(_e) => {
                        println!("loser");
                        std::process::exit(0x1);
                    }
                }
            } else {
                println!("bruh gtfo my system");
                std::process::exit(0x1);
            }
        }
        Err(_e) => {
            println!(
                "failed to get users!!! :( are you sure /etc/mochaexec.d/responsible_adults exists?"
            );
            std::process::exit(0x1);
        }
    }

    if setuid(Uid::from_raw(0)).is_err() {
        println!(
            "setuid failed!!! are you sure {} has the correct permissions?",
            branding::PROJECT_NAME
        );
        println!(
            "{}'s binary should have been set up with the perms 4755 (rwsr-xr-x)",
            branding::PROJECT_NAME
        );
        std::process::exit(0x1);
    }

    let args: Vec<CString> = std::env::args()
        .skip(1)
        .map(|arg| CString::new(arg).unwrap_or_else(|_| std::process::exit(0x2)))
        .collect();

    if args.is_empty() {
        std::process::exit(0x0);
    }

    let command = args[0].clone();

    if execvp(&command, &args).is_err() {
        std::process::exit(0x3);
    }
    Ok(())
}
