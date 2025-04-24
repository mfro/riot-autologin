#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command},
};

use fslock::LockFile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RiotGamesPrivateSettings {
    #[serde(rename = "riot-login")]
    riot_login: RiotLogin,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RiotLogin {
    persist: Option<RiotPersist>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RiotPersist {}

struct RiotClient {
    data_directory: PathBuf,
    config_directory: PathBuf,
    install_directory: PathBuf,
}

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

impl RiotClient {
    fn lock_file(&self) -> PathBuf {
        self.config_directory.join("lockfile")
    }

    fn executable(&self) -> PathBuf {
        self.install_directory.join("RiotClientServices.exe")
    }

    fn launch(&self) -> io::Result<Child> {
        let child = Command::new(self.executable())
            .current_dir(&self.install_directory)
            .spawn()?;

        Ok(child)
    }
}

fn is_data_directory_active(data_directory: impl AsRef<Path>) -> Result<bool> {
    let settings_path = data_directory
        .as_ref()
        .join("RiotGamesPrivateSettings.yaml");

    let mut settings_bytes = vec![];
    File::open(settings_path)?.read_to_end(&mut settings_bytes)?;

    let settings: RiotGamesPrivateSettings = serde_yaml::from_slice(&settings_bytes)?;
    Ok(settings.riot_login.persist.is_some())
}

fn run(client: &RiotClient, path: impl AsRef<Path>) -> Result<()> {
    println!("launch with {:?}", path.as_ref());
    let lock_file = client.lock_file();

    let mut lock = LockFile::open(&lock_file)?;
    if !lock.try_lock()? {
        return Err("could not acquire lock".into());
    }

    if client.data_directory.exists() {
        if is_data_directory_active(&client.data_directory)? {
            return Err("conflict".into());
        } else {
            fs::remove_dir_all(&client.data_directory)?;
        }
    }

    if path.as_ref().exists() {
        fs::rename(path.as_ref(), &client.data_directory)?;
    }
    lock.unlock()?;

    let mut child = client.launch()?;
    println!("{:?}", child.id());

    let status = child.wait()?;
    println!("{:?}", status);

    fs::rename(&client.data_directory, path.as_ref())?;

    Ok(())
}

fn main() -> std::result::Result<(), ()> {
    let client = RiotClient {
        data_directory: PathBuf::from(r"C:\Users\Max\AppData\Local\Riot Games\Riot Client\Data"),
        config_directory: PathBuf::from(
            r"C:\Users\Max\AppData\Local\Riot Games\Riot Client\Config",
        ),
        install_directory: PathBuf::from(r"C:\Riot Games\Riot Client"),
    };

    let arg = std::env::args().nth(1);
    match arg {
        None => {
            println!("invalid account");
            return Err(());
        }

        Some(source_name) => {
            let source_path = client.data_directory.with_extension(source_name);

            if let Err(e) = run(&client, source_path) {
                println!("{:?}", e);
                return Err(());
            }
        }
    }

    Ok(())
}
