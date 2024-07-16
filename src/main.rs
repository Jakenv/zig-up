use core::fmt;
use serde::Deserialize;
use std::process::{Command, Stdio};

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

#[warn(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
enum Architecture {
    x86_64_macos,
    aarch64_macos,
}

impl Architecture {
    const VARIANTS_MORE_MORE: &'static [Architecture] = &[Self::x86_64_macos, Self::aarch64_macos];
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

fn get_latest(archi: &str) {
    let response = reqwest::blocking::get(ZIG_LINK).unwrap();
    let var: Obj = response.json().unwrap();
    match archi {
        "linux" => {
            Command::new("wget")
                .arg(var.master.x86_64_linux.tarball)
                .arg("--progress=bar:force:noscroll")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .spawn()
                .expect("Failed");
        }
        "x86" => {
            Command::new("wget")
                .arg(var.master.x86_64_macos.tarball)
                .arg("--progress=bar:force:noscroll")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .spawn()
                .expect("Failed");
        }
        "arm" => {
            Command::new("wget")
                .arg(var.master.aarch64_macos.tarball)
                .arg("--progress=bar:force:noscroll")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .spawn()
                .expect("Failed");
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
                Select::new("Select your system", MenuInsideMenu::VARIANTS_MORE.to_vec())
                    .with_page_size(9)
                    .prompt()
                    .unwrap_or_else(|_| std::process::exit(0));
            match system_choice {
                MenuInsideMenu::Linux => get_latest("linux"),
                MenuInsideMenu::Mac => {
                    let archi = Select::new(
                        "Select your architecture",
                        Architecture::VARIANTS_MORE_MORE.to_vec(),
                    )
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
