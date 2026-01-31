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

use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};
use nix::unistd::{Uid, User, execvp, setuid};
use pam_client::conv_mock::Conversation;
use pam_client::{Context, Flag};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufReader, Write};
use strfmt::strfmt;
use termion::input::TermRead;
use zeroize::Zeroize;

#[derive(Deserialize)]
#[serde(default)]
struct Config {
    prompt: String,
    prompt_start_color: [u8; 3],
    prompt_end_color: [u8; 3],
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prompt: "{shorthand} {version} | {username}: ".to_string(),
            prompt_start_color: [244, 10, 10],
            prompt_end_color: [255, 40, 40],
        }
    }
}

fn load_config(config: &str) -> Result<Config, Box<dyn std::error::Error>> {
    match toml::from_str::<Config>(&config) {
        Ok(config) => Ok(config),
        Err(e) => {
            eprintln!("failed to parse config AHHHH!!! {}", e);
            Ok(Config::default())
        }
    }
}

fn gradient_text(text: &str, start_rgb: (u8, u8, u8), end_rgb: (u8, u8, u8)) -> String {
    let len = text.chars().count().max(1) as f32;
    if len < 2.0 {
        let (r, g, b) = end_rgb;
        format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
    } else {
        text.chars()
            .enumerate()
            .map(|(i, c)| {
                let ratio = i as f32 / (len - 1.0);
                let r =
                    (start_rgb.0 as f32 + (end_rgb.0 as f32 - start_rgb.0 as f32) * ratio) as u8;
                let g =
                    (start_rgb.1 as f32 + (end_rgb.1 as f32 - start_rgb.1 as f32) * ratio) as u8;
                let b =
                    (start_rgb.2 as f32 + (end_rgb.2 as f32 - start_rgb.2 as f32) * ratio) as u8;
                format!("\x1b[38;2;{r};{g};{b}m{c}\x1b[0m")
            })
            .collect::<String>()
    }
}

fn prompt_password(prompt: &str) -> io::Result<String> {
    let mut password = String::new();
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    let tty_reader = BufReader::new(tty.try_clone()?);

    let mut term = tcgetattr(&tty)?;
    let original = term.clone();

    term.local_flags.remove(LocalFlags::ECHO);
    term.local_flags.remove(LocalFlags::ICANON);

    tcsetattr(&tty, SetArg::TCSANOW, &term)?;

    write!(tty, "{}", format!("{}", prompt))?;
    tty.flush()?;

    for key in tty_reader.keys() {
        let key = key.map_err(|e| io::Error::new(io::ErrorKind::Other, e));
        match key {
            Ok(termion::event::Key::Char('\n')) | Ok(termion::event::Key::Char('\r')) => break,
            Ok(termion::event::Key::Backspace) => {
                if password.pop().is_some() {
                    write!(tty, "\x08")?;
                    tty.flush()?;
                }
            }
            Ok(termion::event::Key::Char(c)) => {
                password.push(c);
                write!(tty, "*")?;
                tty.flush()?;
            }
            Err(e) => return Err(e),
            _ => {}
        }
    }

    tcsetattr(&tty, SetArg::TCSANOW, &original)?;

    writeln!(tty, "{}", format!("\r\x1b[2K{}\r", prompt))?;

    Ok(password)
}

fn authenticate(username: &str, config: &Config) -> Result<(), pam_client::Error> {
    let retries = 5;
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
        let mut password = prompt_password(&gradient_text(
            &*strfmt(&*config.prompt, &vars).unwrap(),
            <(u8, u8, u8)>::from(config.prompt_start_color),
            <(u8, u8, u8)>::from(config.prompt_end_color),
        ))
        .unwrap(); // i don't care if this panics i give up trying to use the stupid question mark
        let conv = Conversation::with_credentials(username, &password);
        let mut context = Context::new("login", Some(username), conv)?;

        match context.authenticate(Flag::NONE) {
            Ok(_) => {
                print!("\x1b[2K\r");
                io::stdout().flush().unwrap();
                return Ok(());
            }
            Err(e) => last_err = Some(e),
        }

        password.zeroize();

        print!("\x1b[2K\r");
        io::stdout().flush().unwrap();

        if attempt < retries {
            eprintln!("wrong lmao")
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
    if nix::unistd::getuid() != Uid::from_raw(0) {
        let config: Config;
        match fs::read_to_string(format!("{}/config.toml", branding::CONFIG_DIR)) {
            Ok(content) => {
                config = load_config(&content)?;
            }
            Err(_e) => {
                eprintln!("no config file!!!! that means your setup is likely BROKEN!!!! AHHH");
                config = load_config("")?;
            }
        }

        match read_allowed_users(&format!("{}/responsible_adults", branding::CONFIG_DIR)) {
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
                    eprintln!("bruh gtfo my system");
                    std::process::exit(0x1);
                }
            }
            Err(_e) => {
                eprintln!(
                    "failed to get users!!! :( are you sure {}/responsible_adults exists?",
                    branding::CONFIG_DIR
                );
                std::process::exit(0x1);
            }
        }
    }

    if setuid(Uid::from_raw(0)).is_err() {
        eprintln!(
            "setuid failed!!! are you sure {} has the correct permissions?",
            branding::PROJECT_NAME
        );
        eprintln!(
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
