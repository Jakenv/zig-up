use core::fmt;
use error_chain::error_chain;
use inquire::{Confirm, Select};
use serde::Deserialize;
use simple_home_dir::home_dir;
use std::{
    fs::{self, File},
    io::BufReader,
    path::Path,
    process::exit,
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
    X86_64Macos,
    Aarch64Macos,
}

impl Architecture {
    const ARCHI: &'static [Architecture] = &[Self::X86_64Macos, Self::Aarch64Macos];
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::X86_64Macos => write!(f, "x86_64-macos"),
            Self::Aarch64Macos => write!(f, "aarch64_macos"),
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

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

async fn download_tar(target: &String) -> Result<()> {
    let tmp_path = Path::new("/tmp/");
    let response = reqwest::get(target).await?;

    let mut dest = {
        let fname = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or("tmp.bin");
        let fname = tmp_path.join(fname);
        File::create(fname)?
    };
    let content = response.bytes().await?;
    let mut content_cursos = std::io::Cursor::new(content);

    std::io::copy(&mut content_cursos, &mut dest)?;
    Ok(())
}

async fn utar_bin(target: String) -> Result<()> {
    let home = home_dir().unwrap();
    let install_path = String::from(home.to_string_lossy() + "/.zig/");

    let ans = Confirm::new("Want to unwrap to default?")
        .with_default(true)
        .with_help_message(&install_path)
        .prompt();
    match ans {
        Ok(true) => (),
        Ok(false) => {
            println!("You can find tar in /tmp/ then");
            download_tar(&target).await?;
            exit(0);
        }
        Err(_) => exit(1),
    }

    download_tar(&target).await?;

    let zig_tar: Vec<&str> = target.split("builds/").collect();
    if let Some(tar_zig) = zig_tar.get(1) {
        let path = Path::new("/tmp/").join(tar_zig).canonicalize()?;
        let file = File::open(path)?;

        let tar = XzDecoder::new(BufReader::new(file));
        let mut utar = Archive::new(tar);

        if !Path::new(&install_path).try_exists()? {
            fs::create_dir(&install_path)?;
        }
        Ok(utar.unpack(install_path)?)
    } else {
        panic!("Error while untaring archive");
    }
}

async fn get_latest(archi: &str) {
    let response = reqwest::get(ZIG_LINK).await.unwrap();
    let var: Obj = response.json().await.unwrap();
    match archi {
        "linux" => {
            utar_bin(var.master.x86_64_linux.tarball)
                .await
                .unwrap_or_else(|e| println!("{}", e));
        }
        "x86" => {
            utar_bin(var.master.x86_64_macos.tarball)
                .await
                .unwrap_or_else(|e| println!("{}", e));
        }
        "arm" => {
            utar_bin(var.master.aarch64_macos.tarball)
                .await
                .unwrap_or_else(|e| println!("{}", e));
        }
        _ => exit(1),
    }
}

#[tokio::main]
async fn main() {
    let choice: Menu = Select::new("Select your action:", Menu::VARIANTS.to_vec())
        .with_page_size(9)
        .prompt()
        .unwrap_or_else(|_| exit(0));

    match choice {
        Menu::Zig => {
            let system_choice: MenuInsideMenu =
                Select::new("Select your system", MenuInsideMenu::SYSTEMS.to_vec())
                    .with_page_size(9)
                    .prompt()
                    .unwrap_or_else(|_| exit(0));

            match system_choice {
                MenuInsideMenu::Linux => get_latest("linux").await,
                MenuInsideMenu::Mac => {
                    let archi =
                        Select::new("Select your architecture", Architecture::ARCHI.to_vec())
                            .with_page_size(9)
                            .prompt()
                            .unwrap_or_else(|_| exit(0));

                    match archi {
                        Architecture::X86_64Macos => get_latest("x86").await,
                        Architecture::Aarch64Macos => get_latest("arm").await,
                    }
                }
            }
        }
    }
}
