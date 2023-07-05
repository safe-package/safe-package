use std::{fs::File, io::BufReader};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json;

// Configuration struct, populated with serde_json and clap.
#[derive(Parser, PartialEq)]
#[command(author = "Mike Doyle", version, about = "Courtesy of [Arnica].io")]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// The package manager to execute. If none is defined, the first ARG
    /// will be used.
    #[arg(short, long)]
    pub exe: Option<String>,

    /// The directory to chroot to.
    #[arg(short, long)]
    pub root_dir: Option<String>,

    /// A list of enviornment variables that the package manager needs.
    #[arg(short, long)]
    pub keep_env: Option<Vec<String>>,

    /// Who to run the package manager as.
    #[arg(short, long)]
    pub user: Option<String>,

    /// Arguments to the package manager.
    pub exe_args: Vec<String>,
}

impl Config {
    pub fn overlay(mut self, other: Config) -> Self {
        self.keep_env = match self.keep_env {
            Some(mut k) => {
                match other.keep_env {
                    Some(mut l) => {
                        k.append(&mut l);
                        k.sort();
                        k.dedup();
                        Some(k)
                    },
                    None => Some(k),
                }
            },
            None => {
                match other.keep_env {
                    Some(k) => Some(k),
                    None => None,
                }
            }
        };

        self.exe_args.extend(other.exe_args.into_iter());

        Self {
            exe: other.exe.or(self.exe),
            root_dir: other.root_dir.or(self.root_dir),
            keep_env: self.keep_env,
            user: other.user.or(self.user),
            exe_args: self.exe_args,
        }
    }
}



pub fn from_filename(fname: &str) -> Option<Config> {
     match File::open(fname) {
        Err(_) => return None,
        Ok(f) => {
            let reader = BufReader::new(f);
            match serde_json::from_reader(reader) {
                Err(e) => {
                    panic!("Error parsing {}: {}", fname, e);
                }
                Ok(config) => {
                    return Some(config);
                }
            };
        },
    };
}

pub fn from_str(content: &str) -> Config {
    let c = serde_json::from_str(content).unwrap();
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        // Define config json string
        let config_string = r#"
            {
                "exe": "/usr/bin/pacman",
                    "user": "nobody",
                    "keep_env": [ "HOME", "PATH" ], 
                "root_dir": "/cellblock/pip3",
                "exe_args": [ ]
            }
        "#;

        // Define config object
        let config_object = Config{
            exe: Some(String::from("/usr/bin/pacman")),
            root_dir: Some(String::from("/cellblock/pip3")),
            keep_env: Some([
                String::from("HOME"), 
                String::from("PATH")].to_vec()),
            user: Some(String::from("nobody")),
            exe_args: [].to_vec(),
        };

        // Build a config object from the config string
        let config_from_str = from_str(&String::from(config_string));
        // Assert equality ZZZ
        assert_eq!(config_object, config_from_str);
    }
}
