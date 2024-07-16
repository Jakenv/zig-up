use core::fmt;
use serde::Deserialize;
use std::{fmt::Result, process::Command};

use inquire::Select;

const ZIG_LINK: &str = "https://ziglang.org/download/index.json";

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
    const VARIANTS_MORE: &'static [MenuInsideMenu] = &[Self::Linux, Self::Mac];
}

impl fmt::Display for MenuInsideMenu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Linux => write!(f, "Linux"),
            Self::Mac => write!(f, "Mac"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Obj {
    master: Master,
}

#[derive(Debug, Deserialize)]
struct Master {
    version: String,
    #[serde(rename = "x86_64-macos")]
    x86_64_macos: Platform,
}

#[derive(Deserialize, Debug)]
struct Platform {
    tarball: String,
}

fn get_latest() {
    let response = reqwest::blocking::get(ZIG_LINK).unwrap();
    let var: Obj = response.json().unwrap();
    println!("{:#?}", var);
}

fn main() {
    let choice: Menu = Select::new("Select your action:", Menu::VARIANTS.to_vec())
        .with_page_size(9)
        .prompt()
        .unwrap_or_else(|_| std::process::exit(0));
    match choice {
        Menu::Zig => {
            let system_choice: MenuInsideMenu =
                Select::new("Select your system", MenuInsideMenu::VARIANTS_MORE.to_vec())
                    .with_page_size(9)
                    .prompt()
                    .unwrap_or_else(|_| std::process::exit(0));
            match system_choice {
                MenuInsideMenu::Linux => {
                    Command::new("wget")
                        .arg("https://ziglang.org/builds/zig-macos-x86_64-0.14.0-dev.321+888708ec8.tar.xz")
                        .output()
                        .expect("Failed");
                }
                MenuInsideMenu::Mac => get_latest(),
            }
        }
    }
}
