use clap::Parser;
use directories::ProjectDirs;
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, create_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::ClockSequence;
use uuid::{Context, Timestamp, Uuid};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// workflow |
/// init -> create your .sql files -> pg_dump -> migrate |
/// project already init |
/// get last version of db -> setup -> create your .sql files -> pg_dump -> migrate
enum Args {
    /// Init migrust in your current folder
    Init(InitArgs),

    /// Check if you need migration
    Check(CheckArgs),

    /// Do migration if you need to
    /// /!\ need the last version of db before being able to use /!\
    Migrate(MigrateArgs),
}

#[derive(Parser)]
struct InitArgs {}
#[derive(Parser)]
struct SetupArgs {}
#[derive(Parser)]
struct CheckArgs {
    #[arg(default_value = "default")]
    config: String,
}
#[derive(Parser)]
struct MigrateArgs {
    #[arg(default_value = "default")]
    config: String,
}

fn get_current_working_dir() -> String {
    std::env::current_dir()
        .unwrap()
        .as_path()
        .to_str()
        .unwrap()
        .to_string()
}

fn something_exist(path: &str) -> bool {
    let path = Path::new(path);
    path.is_dir() || path.is_file()
}

fn file_modified_time_in_millis(path: &str) -> u128 {
    fs::metadata(path)
        .unwrap()
        .created()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn is_project_folder_inited() -> bool {
    something_exist(&(get_current_working_dir() + "/migrust/backup"))
        && something_exist(&(get_current_working_dir() + "/migrust/migrations"))
        && something_exist(&(get_current_working_dir() + "/migrust/default.json"))
}

fn get_project_config(config_name: &str) -> ProjectConfig {
    if something_exist(&(get_current_working_dir() + "/migrust/" + config_name + ".json")) {
        serde_json::from_str(
            &fs::read_to_string(&(get_current_working_dir() + "/migrust/" + config_name + ".json"))
                .unwrap(),
        )
        .unwrap()
    } else {
        ProjectConfig {
            id: Uuid::new_v1(
                Timestamp::from_rfc4122(
                    14976234442241191232,
                    Context::new(42).generate_sequence(0, 0),
                ),
                &[1, 2, 3, 4, 5, 6],
            )
            .to_string(),
            db: DB {
                database: "".to_string(),
                host: "".to_string(),
                password: "".to_string(),
                user_name: "".to_string(),
                port: 5432,
            },
        }
    }
}

fn get_config(path: &str, config_name: &str) -> Config {
    if something_exist(&(path.to_owned() + "/" + config_name + ".json")) {
        serde_json::from_str(
            &fs::read_to_string(&(path.to_owned() + "/" + config_name + ".json")).unwrap(),
        )
        .unwrap()
    } else {
        Config {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        }
    }
}

fn main() {
    let args = Args::parse();
    let Some(proj_dirs) = ProjectDirs::from("com", "Spinelle", "MigRust") else { return };
    if !something_exist(proj_dirs.config_dir().to_str().unwrap()) {
        println!("Creating save folders structure ...");
        let _ = create_dir_all(proj_dirs.config_dir().to_str().unwrap());
    };
    match args {
        Args::Init(_) => {
            if is_project_folder_inited() {
                return println!("Migrust is already inited in this project");
            } else {
                let conf = get_project_config("default");
                println!("Creating folders structure ...");
                let _ = create_dir_all(&(get_current_working_dir() + "/migrust/backup"));
                let _ = create_dir_all(&(get_current_working_dir() + "/migrust/migrations"));
                let _ = write(
                    &(get_current_working_dir() + "/migrust/default.json"),
                    serde_json::to_string_pretty(&conf).unwrap(),
                );
                println!("Folders structure created !");
            }
        }
        Args::Check(e) => {
            if !is_project_folder_inited() {
                return println!("This folder is not inited");
            }
            let project_config = get_project_config(e.config.as_str());
            let config = get_config(
                &proj_dirs.config_dir().to_str().unwrap(),
                &project_config.id,
            );
            let sql_files: Vec<PathBuf> =
                fs::read_dir(&(get_current_working_dir() + "/migrust/migrations"))
                    .unwrap()
                    .into_iter()
                    .map(|x| x.map(|entry| entry.path()).unwrap())
                    .filter(|x| x.file_name().unwrap().to_str().unwrap().ends_with(".sql"))
                    .filter(|x| {
                        file_modified_time_in_millis(
                            &(get_current_working_dir()
                                + "/migrust/migrations/"
                                + x.file_name().unwrap().to_str().unwrap()),
                        ) > config.time
                    })
                    .collect();
            if sql_files.len() <= 0 {
                return println!("No sql files to migrate ...");
            } else {
                return println!("{:?} files to migrate", sql_files.len());
            }
        }
        Args::Migrate(e) => {
            if !is_project_folder_inited() {
                return println!("This folder is not inited");
            }
            let project_config = get_project_config(e.config.as_str());
            let config = get_config(
                &proj_dirs.config_dir().to_str().unwrap(),
                &project_config.id,
            );
            if !something_exist(
                &(proj_dirs.config_dir().to_str().unwrap().to_string()
                    + "/"
                    + &project_config.id
                    + ".json"),
            ) {
                let _ = write(
                    &(proj_dirs.config_dir().to_str().unwrap().to_string()
                        + "/"
                        + &project_config.id
                        + ".json"),
                    serde_json::to_string_pretty(&config).unwrap(),
                );
            };
            let sql_files: Vec<PathBuf> =
                fs::read_dir(&(get_current_working_dir() + "/migrust/migrations"))
                    .unwrap()
                    .into_iter()
                    .map(|x| x.map(|entry| entry.path()).unwrap())
                    .filter(|x| x.file_name().unwrap().to_str().unwrap().ends_with(".sql"))
                    .filter(|x| {
                        file_modified_time_in_millis(
                            &(get_current_working_dir()
                                + "/migrust/migrations/"
                                + x.file_name().unwrap().to_str().unwrap()),
                        ) > config.time
                    })
                    .collect();
            if sql_files.len() <= 0 {
                return println!("No sql files to migrate ...");
            }
            println!("Backing up db before migration");
            let dump = Command::new("pg_dump")
                .env("PGPASSWORD", &project_config.db.password)
                .args([
                    "-h",
                    &project_config.db.host,
                    "-d",
                    &project_config.db.database,
                    "-U",
                    &project_config.db.user_name,
                    "-p",
                    &project_config.db.port.to_string(),
                    "-f",
                    &(get_current_working_dir()
                        + "/migrust/backup/"
                        + &SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                            .to_string()
                        + ".sql"),
                ])
                .stderr(Stdio::null())
                .stdout(Stdio::null())
                .status()
                .unwrap()
                .code()
                .unwrap();
            if dump != 0 {
                return println!("Error while dumping db");
            }
            for x in sql_files {
                println!(
                    "Start migration of {:?}",
                    x.file_name().unwrap().to_str().unwrap()
                );
                let migration = Command::new("psql")
                    .env("PGPASSWORD", &project_config.db.password)
                    .args([
                        "-h",
                        &project_config.db.host,
                        "-d",
                        &project_config.db.database,
                        "-U",
                        &project_config.db.user_name,
                        "-p",
                        &project_config.db.port.to_string(),
                        "-a",
                        "-w",
                        "-f",
                        &(get_current_working_dir()
                            + "/migrust/migrations/"
                            + &x.file_name().unwrap().to_str().unwrap()),
                        "-v",
                        "ON_ERROR_STOP=1",
                    ])
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .status()
                    .unwrap()
                    .code()
                    .unwrap();
                if migration != 0 {
                    return println!(
                        "Error while migrating file {:?}",
                        x.file_name().unwrap().to_str().unwrap()
                    );
                }
                let _ = write(
                    &(proj_dirs.config_dir().to_str().unwrap().to_string()
                        + "/"
                        + &project_config.id
                        + ".json"),
                    serde_json::to_string_pretty(&Config {
                        time: file_modified_time_in_millis(
                            &(get_current_working_dir()
                                + "/migrust/migrations/"
                                + x.file_name().unwrap().to_str().unwrap()),
                        ),
                    })
                    .unwrap(),
                );
                println!(
                    "Migration of {:?} successful",
                    x.file_name().unwrap().to_str().unwrap()
                );
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    time: u128,
}

#[derive(Serialize, Deserialize)]
struct ProjectConfig {
    id: String,
    db: DB,
}

#[derive(Serialize, Deserialize)]
struct DB {
    database: String,
    user_name: String,
    password: String,
    host: String,
    port: u32,
}
