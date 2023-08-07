use nix::unistd::{execv, setuid, fork, ForkResult, User};
use std::ffi::CString;
use nix::sys::wait::waitpid;
use std::process::exit;
use crate::chroot;

//use std::env;

pub fn drop_privs(user: &str) -> Result<(),&'static str> {
    if user == "" {
        return Ok(())
    }

    if let Ok(Some(user_obj)) = User::from_name(user) {
        if let Ok(_) = setuid(user_obj.uid) {
            Ok(())
        } else {
            Err("failed to setuid. Are you root?")
        }
    } else {
        Err("user not found")
    }
}

    
pub fn exec_pm(path: &str, 
               args: Vec<std::string::String>, 
               user: &str, 
               d: &str,
               bind_mount_rules: &Vec<String>) {

        
    let p = &CString::new(path).unwrap();
    let mut v = Vec::new();
    v.push(p.clone());

    for arg in args {
        v.push(CString::new(arg).unwrap());
    }

    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            waitpid(child, None).unwrap();
            if ! chroot::unbind_mounts(bind_mount_rules, d) {
                eprintln!("Cleanup failed.");
            }
        }

        Ok(ForkResult::Child) => {

            if d != (String::from("/")) {
                match chroot::chroot(&d){
                    Ok(()) => { },
                    Err(e) => {
                        eprintln!("{}", e);
                        exit(1);
                    },
                }
            }

            if let Err(e) = drop_privs(user) {
                panic!("{}", e);
            }

            match execv(p, &v) {
                Err(e) => {
                    eprintln!("Failed to execute {path}: {e}");
                },
                Ok(_) => eprintln!("The impossible has happened."),
            };
        }
        Err(_) => {panic!("Fork failed")} ,
    }
}
