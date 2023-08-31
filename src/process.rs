use anyhow::{anyhow, Result};
use libc;
use std::{
    env,
    io::Read,
    os,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::ui::UI;

enum User {
    Root,
    User,
}

fn run_process_impl(command: &str, user: User, ui: &mut Arc<Mutex<UI>>) -> Result<()> {
    let mut child = match user {
        User::Root => Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("{} 2>&1", command))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?,
        User::User => {
            // We need to switch user
            Command::new("sudo")
                .arg("-u")
                .arg(env::var("SUDO_USER")?)
                .arg("/bin/sh")
                .arg("-c")
                .arg(format!("{} 2>&1", command))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        }
    };

    {
        let mut line: String = String::new();
        let mut buf = [0];
        while child.stdout.as_mut().unwrap().read_exact(&mut buf).is_ok() {
            if buf[0] as char == '\r' {
                ui.lock().unwrap().log().replace_newest(line.clone());
                line.clear();
                continue;
            }
            if buf[0] as char == '\n' {
                ui.lock().unwrap().log().append(line.clone());
                line.clear();
                continue;
            }
            line.push(buf[0] as char);
        }
    }

    match child.wait()?.code() {
        Some(code) => {
            if code == 0 {
                Ok(())
            } else {
                Err(anyhow!("{} failed with return code {}", command, code))
            }
        }
        None => Err(anyhow!("{} got signaled!", command)),
    }
}

pub fn run_process(command: &str, ui: &mut Arc<Mutex<UI>>) -> Result<()> {
    run_process_impl(command, User::Root, ui)
}

pub fn run_process_user(command: &str, ui: &mut Arc<Mutex<UI>>) -> Result<()> {
    run_process_impl(command, User::User, ui)
}
