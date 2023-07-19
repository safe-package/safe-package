use std::os::unix::fs;
use nix::unistd;
use subprocess;
use sys_mount::{Mount, MountFlags};
use regex::Regex;
use debug_print::debug_println;


struct BMDirective {
    local: String,
    mountpoint: String,
    options: Option<String>,
}

fn parse_bm_directive(line: &str) -> Option<BMDirective> {
    //let r = "[ \t]*local:[ \t]+(<local>[^ \t]+)[ \t]+mountpoint:[ \t]+(<mountpoint>[^ \t]+)[ \t]*(opts:[ \t]+(<options>[^ \t]+)){0,1}[ \t]*";
    let r = "[ \t]*local:[ \t]+(?<local>[^ \t]+)[ \t]+mountpoint:[ \t]*(?<mountpoint>[^ \t]+)[ ]*(opts:[ \t]+(?<options>[^ \t]+)[ \t]*){0,1}";

    let rx = Regex::new(r).unwrap();
    let Some(caps) = rx.captures(line) else {return None};
    // let o: Option<String> = Some(caps["options"].to_string()) else { None };

    let o: Option<String> = match caps.name("options") {
        None => None,
        Some(m) => Some(m.as_str()
            .replace("\040", " ")
            .replace("\011", "\t")
            .to_string()
            ),
    };

    let local = caps["local"].to_string()
        .replace("\040", " ")
        .replace("\011", "\t");

    let mountpoint = caps["mountpoint"].to_string()
        .replace("\040", " ")
        .replace("\011", "\t");

    let r = BMDirective{
        local: local,
        mountpoint: mountpoint,
        options: o,
    };

    Some(r)
}

pub fn bind_mounts(bind_mount_rules: &Vec<String>, root: &str) -> bool {

    for line in bind_mount_rules {

        let rule = parse_bm_directive(&line)
            .expect("Couldn't parse bind mount rule: {line}");


        // We need to fully specify the mount point inside of the chroot:
        let mut chroot_mount: String = String::from(root) + &rule.mountpoint;

        let mut flags: MountFlags = MountFlags::empty();
        flags = flags | MountFlags::BIND;

        // If there are no additional options, nothing to do but BIND
        let option_str = rule.options.unwrap_or("".to_string());

        let options = if option_str == "" {
            [].to_vec()
        } else {
            //debug_println!("{}", option_str);
            option_str.split(",").collect()
        };

        for o in options {
            match &o[..] {
                "diratime"      => flags = flags | ! MountFlags::NODIRATIME,
                "nodiratime"    => flags = flags | MountFlags::NODIRATIME,

                "dev"           => flags = flags | ! MountFlags::NODEV,
                "noddev"        => flags = flags | MountFlags::NODEV,

                "dirsync"       => flags = flags | MountFlags::DIRSYNC,
                "nodirsync"     => flags = flags | ! MountFlags::DIRSYNC,

                "exec"          => flags = flags | ! MountFlags::NOEXEC,
                "noexec"        => flags = flags | MountFlags::NOEXEC,

                "mand"          => flags = flags | MountFlags::MANDLOCK,
                "nomand"        => flags = flags | ! MountFlags::MANDLOCK,

                "relatime"      => flags = flags | MountFlags::RELATIME,
                "norelatime"    => flags = flags | ! MountFlags::RELATIME,

                "strictatime"   => flags = flags | MountFlags::STRICTATIME,
                "nostrictatime" => flags = flags | ! MountFlags::STRICTATIME,

                "suid"          => flags = flags | ! MountFlags::NOSUID,
                "nosuid"        => flags = flags | MountFlags::NOSUID,

                "sync"          => flags = flags | MountFlags::SYNCHRONOUS,
                "nosync"        => flags = flags | ! MountFlags::SYNCHRONOUS,

                "ro"            => flags = flags | MountFlags::RDONLY,
                "rw"            => flags = flags | ! MountFlags::RDONLY,

                "default"       => flags = flags | ! MountFlags::RDONLY
                                                | ! MountFlags::NOSUID
                                                | ! MountFlags::NODEV
                                                | ! MountFlags::NOEXEC
                                                | ! MountFlags::SYNCHRONOUS,
                        
                other => { panic!("Mount option not supported: {other}")},
            } // end match
        } // end for

        let mount_result = Mount::builder()
            .flags(flags)
            .mount(rule.local.clone(), &chroot_mount);

        match mount_result {
            Ok(_) => { 
                debug_println!("Mounted {}", rule.local); 
                continue;
            },
            Err(e) => { 
                println!("Failed to mount rule.local: {e}");
                return false;
            },
        }
    }
    return true;
    //match subprocess::Exec::shell("sudo bindmount-pip3.sh").join() {

    //    Ok(subprocess::ExitStatus::Exited(0)) => true,
    //    _ => false,
    //}
}

pub fn unbind_mounts(_bind_mount_rules: &Vec<String>) -> bool {

    match subprocess::Exec::shell("sudo bindumount-pip3.sh").join() {

        Ok(subprocess::ExitStatus::Exited(0)) => true,
        _ => false,
    }
}

pub fn chroot(path: &str) -> Result<(),&'static str> {

    if ! unistd::geteuid().is_root() {
        return Err("You must be root to set a root-dir. Configure a 'user' to drop privs after chrooting.");
    }

    fs::chroot(path).expect("Failed to chroot");
    
    std::env::set_current_dir("/").expect("Failed to change directory");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_bind_roots() {
        assert!(bind_mounts(&[].to_vec()))
    }

    #[test]
    fn test_unbind_roots() {
        assert!(unbind_mounts(&[].to_vec()))
    }
}
