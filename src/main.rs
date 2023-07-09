use std::env;
use clap::Parser;
use std::process::exit;
use debug_print::debug_println;
use nix::unistd;



mod exec;           // look in exec.rs
mod environment;    // look in environment.rs
mod config;         // look in config.rs
mod chroot;         // look in chroot.rs

fn main() {

    // Strategy: 
    // 0. Gather configuration from various sources.
    // 1. Clear the environment variables.
    // 2. Isolate the filesystem.
    // 3. Drop privileges.
    // 4. Execute the package manager.
    // 5. Clean up.


    // 0. Gather configuration from various sources.
    // First, here are our defaults.
    let mut config = config::Config{
        exe: None,
        root_dir:  Some(String::from("/")),
        keep_env: Some([].to_vec()),
        user: None,
        bind_mounts: [].to_vec(),
        exe_args: [].to_vec(),
    };

    // Next we look in the system-specific config in /etc.
    let etc_filename = String::from("/etc/safe-package/config.json");
    config = match config::from_filename(&etc_filename) {
        None => config,
        Some(c) => config.overlay(c),
    };


    // Next we look in the user's .safe-package directory.
    let user_filename = match env::var("HOME") {
        // Most unixen
        Ok(val) => format!("{val}/.safe-package/config.json"),
        // Some single-user embedded systems
        Err(_e) => String::from("/.safe-package/config.json"),
    };
    config = match config::from_filename(&user_filename) {
        None => config,
        Some(c) => config.overlay(c),
    };

    // Next, let's see if the current working directory has config.
    let cwd_filename = String::from("./.safe-package/config.json");
    config = match config::from_filename(&cwd_filename) {
        None => config,
        Some(c) => config.overlay(c),
    };

    // Finally, let's check the command line arguments.
    config = config.overlay(config::Config::parse());

    debug_println!("{:?}", config);



    // 1. Clear the environment variables.
    match config.keep_env {
        None => { 
            environment::clear_env(&[ ].to_vec());
        },
        Some(k) => {
            environment::clear_env(&k);
        },
    }

    // 2. Isolate the filesystem.
    if let Some(d) = config.root_dir {

        // Do we have bind mount rules? If so, let's
        // process them.
        chroot::bind_mounts(&config.bind_mounts);

        if d != (String::from("/")) {
            match chroot::chroot(&d) {
                Ok(()) => { },
                Err(e) => {
                    eprintln!("{}", e);
                    exit(1);
                },
            }
        }
    }

    // 3. Drop privileges
    match config.user {
        None => {
            if unistd::geteuid().is_root() {
                println!("{} {}", 
                         "Warning! Running as root.", 
                         "I hope you know what you're doing.");
            }
        }, 
        Some(user) => {
            match exec::drop_privs(&user) {
                Ok(()) => { },
                Err(e) => {
                    eprintln!("Couldn't drop privileges to user {}: {}",
                        user, e);
                    exit(1);

                },
            }
        },
    }

    // 4. Execute the package manager.
    match config.exe {
        Some(e) => { 
           exec::exec_pm(&e, config.exe_args.to_vec());
        },
        None => {
            if config.exe_args.len() > 0 {
                let exe = config.exe_args[0].clone();
                config.exe_args = config.exe_args[1..].to_vec();
                exec::exec_pm(&exe, config.exe_args.to_vec());
            } else {
                panic!("Nothing to execute!");
            }
        },
    }

    // 5. Clean up.
    // If there are bind mounts configured, we should clean those up.
    chroot::unbind_mounts(&config.bind_mounts);
}
