use anyhow::anyhow;
use core::fmt;
use thiserror::Error;
use futures_util::{future, StreamExt};
use inquire::{Confirm, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use simple_home_dir::home_dir;
use std::{
    fs::{self, File}, io::{BufReader, BufWriter, Write}, path::{Path, PathBuf}, process::exit, time::Duration
};
use tokio::sync::OnceCell;
use tar::Archive;
use xz::bufread::XzDecoder;

const ZIG_LINK: &str = "https://ziglang.org/download/index.json";
static PROGRESS_BAR_STYLE: OnceCell<ProgressStyle> = OnceCell::const_new();

// TODO add Zls or other stuff
#[derive(Debug, Copy, Clone)]
enum Menu {
    Zig,
    Quit,
}

impl Menu {
    const VARIANTS: &'static [Menu] = &[Self::Zig, Self::Quit];
}

impl fmt::Display for Menu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Zig => write!(f, "Dowload latest Zig binary"),
            Self::Quit => write!(f, "Quit"),
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

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

async fn download_tar(target: &str) -> Result<PathBuf, Error> {
    let tmp_path = Path::new("/tmp/");
    let response = reqwest::get(target).await?;
    let total_size = response
        .content_length()
        .ok_or_else(|| anyhow!("Missing content lenght!"))?;

    let filename = response
        .url()
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|name| !name.is_empty())
        .unwrap_or("tmp.bin");

    let filepath = tmp_path.join(filename);
    let file = File::create(&filepath)?;
    let mut dest = BufWriter::new(file);

    let pb = ProgressBar::new(total_size);
    pb.set_style(get_bar_style().await);

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        pb.inc(chunk.len() as u64);
        dest.write_all(&chunk)?;
    }
    pb.finish_with_message("Download complete");

    Ok(filepath)
}

async fn utar_bin(target: String) -> Result<(), Error> {
    let home = home_dir().unwrap();
    let install_path = String::from(home.to_string_lossy() + "/.zig/");
    if !confirm_unpack(&install_path)? {
        download_tar(&target).await?;
        println!("You can find tar in /tmp/ then");
        exit(0);
    }
    let tar_path = download_tar(&target).await?;
    extract_tarball(&install_path, tar_path)?;
    Ok(())
}

fn confirm_unpack(install_path: &str) -> Result<bool, Error> {
    let ans = Confirm::new("Want to unwrap to default?")
        .with_default(true)
        .with_help_message(install_path)
        .prompt();
    match ans {
        Ok(true) => Ok(true),
        Ok(false) => Ok(false),
        Err(_) => exit(1),
    }
}

fn extract_tarball(install_path: &str, tar_path: PathBuf) -> Result<(), Error> {
    let file = File::open(tar_path)?;
    let tar = XzDecoder::new(BufReader::new(file));
    let mut utar = Archive::new(tar);

    if !Path::new(&install_path).try_exists()? {
        fs::create_dir(install_path)?;
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{msg}\n{spinner:.green} [{elapsed_precise}] Extracting...")
            .unwrap()
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    for entry in utar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.display().to_string();
        pb.set_message(format!("Extracting: {path}"));
        entry.unpack_in(install_path)?;
    }
    pb.finish_with_message("Extraction complete");
    Ok(())
}

async fn get_bar_style() -> ProgressStyle {
    PROGRESS_BAR_STYLE.get_or_init(|| {
        future::ready(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})").unwrap()
            .progress_chars("#>-"))
    }).await.clone()
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
        "arm" => {
            utar_bin(var.master.aarch64_macos.tarball)
                .await
                .unwrap_or_else(|e| println!("{}", e));
        }
        "x86" => {
            utar_bin(var.master.x86_64_macos.tarball)
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
                        Architecture::Aarch64Macos => get_latest("arm").await,
                        Architecture::X86_64Macos => get_latest("x86").await,
                    }
                }
            }
        },
        Menu::Quit => {
            exit(0)
        }
    }
}
