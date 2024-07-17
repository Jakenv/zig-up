use core::fmt;
use inquire::{Confirm, Select};
use serde::Deserialize;
use std::{
    fs::File,
    io::{self, BufReader},
    path::Path,
    process::{Command, Stdio},
};
use tar::Archive;
use xz::bufread::XzDecoder;

const ZIG_LINK: &str = "https://ziglang.org/download/index.json";

// TODO add Zls or other stuff
#[derive(Debug, Copy, Clone)]
enum Menu {
    Zig,
}

impl Menu {
    const VARIANTS: &'static [Menu] = &[Self::Zig];
}

impl fmt::Display for Menu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Zig => write!(f, "Dowload latest Zig binary"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum MenuInsideMenu {
    Linux,
    Mac,
}

impl MenuInsideMenu {
    const SYSTEMS: &'static [MenuInsideMenu] = &[Self::Linux, Self::Mac];
}

impl fmt::Display for MenuInsideMenu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Linux => write!(f, "Linux"),
            Self::Mac => write!(f, "Mac"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Architecture {
    x86_64_macos,
    aarch64_macos,
}

impl Architecture {
    const ARCHI: &'static [Architecture] = &[Self::x86_64_macos, Self::aarch64_macos];
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::x86_64_macos => write!(f, "x86_64-macos"),
            Self::aarch64_macos => write!(f, "aarch64_macos"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Obj {
    master: Master,
}

#[derive(Debug, Deserialize)]
struct Master {
    #[serde(rename = "x86_64-macos")]
    x86_64_macos: Platform,
    #[serde(rename = "aarch64-macos")]
    aarch64_macos: Platform,
    #[serde(rename = "x86_64-linux")]
    x86_64_linux: Platform,
}

#[derive(Deserialize, Debug)]
struct Platform {
    tarball: String,
}

fn call_wget(target: &String) {
    Command::new("wget")
        .arg(target)
        .args(["-P", "/tmp/"])
        .spawn()
        .expect("Failed");
}

fn utar_bin(target: String) -> Result<(), std::io::Error> {
    let mut install_path = String::from("~/.zig/");
    let ans = Confirm::new("Want to unwrap to default?")
        .with_default(true)
        .with_help_message("Default is ~/.zig/")
        .prompt();
    match ans {
        Ok(true) => install_path = String::from("~/.zig/"),
        Ok(false) => {
            io::stdin()
                .read_line(&mut install_path)
                .expect("Failed to read line");
        }
        Err(_) => install_path = String::from("~/.zig/"),
    }

    let zig_tar: Vec<&str> = target.split("builds/").collect();
    if let Some(tar_zig) = zig_tar.get(1) {
        let path = Path::new("/tmp/").join(tar_zig).canonicalize()?;
        let file = File::open(path)?;

        let tar = XzDecoder::new(BufReader::new(file));
        let mut utar = Archive::new(tar);
        Ok(utar.unpack(install_path)?)
    } else {
        panic!("chuj");
    }
}

fn get_latest(archi: &str) {
    // TODO Make it async
    let response = reqwest::blocking::get(ZIG_LINK).unwrap();
    let var: Obj = response.json().unwrap();
    match archi {
        "linux" => {
            call_wget(&var.master.x86_64_linux.tarball);
            let _ = utar_bin(var.master.x86_64_linux.tarball);
        }
        "x86" => {
            call_wget(&var.master.x86_64_macos.tarball);
            let _ = utar_bin(var.master.x86_64_macos.tarball);
        }
        "arm" => {
            call_wget(&var.master.aarch64_macos.tarball);
            let _ = utar_bin(var.master.aarch64_macos.tarball);
        }
        _ => std::process::exit(0),
    }
}

fn main() {
    let choice: Menu = Select::new("Select your action:", Menu::VARIANTS.to_vec())
        .with_page_size(9)
        .prompt()
        .unwrap_or_else(|_| std::process::exit(0));
    match choice {
        Menu::Zig => {
            let system_choice: MenuInsideMenu =
                Select::new("Select your system", MenuInsideMenu::SYSTEMS.to_vec())
                    .with_page_size(9)
                    .prompt()
                    .unwrap_or_else(|_| std::process::exit(0));
            match system_choice {
                MenuInsideMenu::Linux => get_latest("linux"),
                MenuInsideMenu::Mac => {
                    let archi =
                        Select::new("Select your architecture", Architecture::ARCHI.to_vec())
                            .with_page_size(9)
                            .prompt()
                            .unwrap_or_else(|_| std::process::exit(0));
                    match archi {
                        Architecture::x86_64_macos => get_latest("x86"),
                        Architecture::aarch64_macos => get_latest("arm"),
                    }
                }
            }
        }
    }
}
