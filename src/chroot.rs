use std::os::unix;
use nix::unistd;
use regex::Regex;
use libmount::{BindMount, Remount};
use nix::mount::{MsFlags, mount, umount, umount2, MntFlags};
use std::ffi::CStr;
use std::fs;

#[derive(Debug, Default)]
struct MountOptions {
    nodiratime:     bool,
    nodev:          bool,
    dirsync:        bool,
    noexec:         bool,
    mandlock:       bool,
    relatime:       bool,
    strictatime:    bool,
    nosuid:         bool,
    synchronous:    bool,
    rdonly:         bool,
}



struct BMDirective {
    local: String,
    mountpoint: String,
    options: Option<String>,
}

fn rbind_make_rslave(root: &str, dir: &str) -> Result<(),&'static str> {
    let mut flags = MsFlags::MS_BIND | MsFlags::MS_REC;

    let tgt = format!("{}{}", root, dir);

    if let Err(_) = mount(Some(&*dir), tgt.as_str(),
          None::<&CStr>, flags, None::<&CStr>) {
        return Err("Failed to mount.");
    }

    // For reasons that I would deem under-documented, you can't
    // just mount once with MS_REC | MS_SLAVE | MS_BIND. You have
    // to mount dev then change the propagation, otherwise
    // $root/dev/pts gets stuck and you can't unmount it.
    // Note this is two calls to mount, not a remount (a la MS_REMOUNT).
    let none = String::from("none");

                              // anytime you want to change this,
                              // kernel devs, I'm here for you.
    flags = MsFlags::MS_REC | MsFlags::MS_SLAVE;

    if let Err(_) = mount(Some(&*none), tgt.as_str(),
          None::<&CStr>, flags, None::<&CStr>) {
        return Err("Failed to change propagation");
    }

    Ok(())
}


fn mount_dev_proc_sys(root: &str) -> bool {

    let procsrc = String::from("/proc");
    let proctgt = format!("{}/proc", root);

    if let Err(err) = mount(Some(&*procsrc), proctgt.as_str(),
                            None::<&CStr>, MsFlags::MS_BIND, None::<&CStr>) {
        println!("Error mounting proc: {}", err);
        return false;
    }

    if let Err(err) = rbind_make_rslave(root, &"/dev") {
        println!("Error mounting dev: {}", err);
        return false;
    }

    if let Err(err) = rbind_make_rslave(root, &"/sys") {
        println!("Error mounting sys: {}", err);
        return false;
    }

    true

}

fn read_mounts_from_proc_self() -> Vec<String> {

    let mut v: Vec<String> = Vec::new();

    let mountinfo = String::from("/proc/self/mountinfo");
    let contents = fs::read_to_string(mountinfo)
        .expect("should have been able to read {mountinfo}");

    for line in contents.split("\n") {
        let fields: Vec<&str> = line.split(" ").collect::<Vec<&str>>();
        if fields.len() > 5 {
            v.push(fields[4].to_owned());
        }
    }

    v
}

fn recursive_umount(tgt: &str, opt_mounts: Option<Vec<String>>) -> bool {
    let mut r = true;
    let mut mounts = opt_mounts.unwrap_or(read_mounts_from_proc_self());
    mounts.sort();
    mounts.reverse();

    for mount in mounts {
        if mount.starts_with(tgt) {
            if let Err(e) = umount2(mount.as_str(), MntFlags::MNT_FORCE) {
                println!("Error unmounting {}: {}", mount, e);
                r = false;
            }
        }
    }
    return r;
}
fn unmount_dev_proc_sys(root: &str) -> bool {

    let proctgt = format!("{}/proc", root);
    if ! recursive_umount(&proctgt, None) {
        return false
    }

    let devtgt = format!("{}/dev", root);

    if ! recursive_umount(&devtgt, None) {
        return false;
    }

    let systgt = format!("{}/sys", root);
    if ! recursive_umount(&systgt, None) {
        return false;
    }
    true
}

fn parse_bm_directive(line: &str) -> Option<BMDirective> {
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

    if ! mount_dev_proc_sys(root) {
        return false;
    }

    for line in bind_mount_rules {

        let rule = parse_bm_directive(&line)
            .expect("Couldn't parse bind mount rule: {line}");


        // We need to fully specify the mount point inside of the chroot:
        let chroot_mount: String = String::from(root) + &rule.mountpoint;

        let mut flags = MountOptions{
            nodiratime: false, nodev: false, dirsync: false, 
            noexec: false, mandlock: false, relatime: false, 
            strictatime: false, nosuid: false, synchronous: false, 
            rdonly: false};


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
                "diratime"      => flags.nodiratime = false,
                "nodiratime"    => flags.nodiratime = true,

                "dev"           => flags.nodev = false,
                "nodev"        => flags.nodev = true,

                "dirsync"       => flags.dirsync = true,
                "nodirsync"     => flags.dirsync = false,

                "exec"          => flags.noexec = false,
                "noexec"        => flags.noexec = true,

                "mand"          => flags.mandlock = true,
                "nomand"        => flags.mandlock = false,

                "relatime"      => flags.relatime = true,
                "norelatime"    => flags.relatime = false,

                "strictatime"   => flags.strictatime = true,
                "nostrictatime" => flags.strictatime = false,

                "suid"          => flags.nosuid = false,
                "nosuid"        => flags.nosuid = true,

                "sync"          => flags.synchronous = true,
                "nosync"        => flags.synchronous = false,

                "ro"            => flags.rdonly = true,
                "rw"            => flags.rdonly = false,

                "default"       => {
                    flags.rdonly = true;
                    flags.nosuid = true;
                    flags.noexec = true;
                    flags.synchronous = true;
                },
                        
                other => { panic!("Mount option not supported: {other}")},
            } // end match
        } // end for
         
        //debug_println!("Flags: {:?}", flags);

        match BindMount::new(rule.local.clone(), &chroot_mount).mount() {
            Ok(_) => { },
            Err(e) => {
                println!("Failed to mount {} to {}: {}",
                         rule.local,
                         chroot_mount,
                         e);
                return false;
            },
        };

        match Remount::new(&chroot_mount)
            .bind(true)
            .readonly(flags.rdonly)
            .noexec(flags.noexec)
            .nosuid(true)
            .remount() {
            Ok(_) => { },
            Err(e) => {
                println!("Failed to mount {} to {}: {}",
                         rule.local,
                         chroot_mount,
                         e);
                return false;
            },
        };

    }
    return true;
    //match subprocess::Exec::shell("sudo bindmount-pip3.sh").join() {

    //    Ok(subprocess::ExitStatus::Exited(0)) => true,
    //    _ => false,
    //}
}

pub fn unbind_mounts(bind_mount_rules: &Vec<String>, root: &str) -> bool {

    if ! unmount_dev_proc_sys(root) {
        return false;
    }

    for line in bind_mount_rules {

        let rule = parse_bm_directive(&line)
            .expect("Couldn't parse bind mount rule: {line}");

        let chroot_mount: String = String::from(root) + &rule.mountpoint;

        match umount(chroot_mount.as_str()) {
            Ok(_) => { },
            Err(e) => {
                println!("Failed to unmount {}: {}",
                         chroot_mount,
                         e);
                return false;
            },
        };
    }
    return true;
}

pub fn chroot(path: &str) -> Result<(),&'static str> {

    if ! unistd::geteuid().is_root() {
        return Err("You must be root to set a root-dir. Configure a 'user' to drop privs after chrooting.");
    }

    unix::fs::chroot(path).expect("Failed to chroot");
    
    std::env::set_current_dir("/").expect("Failed to change directory");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_bind_roots() {
        let rule = String::from("local: /usr/lib mountpoint: /usr/lib opts: ro");
        assert!(bind_mounts(&[rule].to_vec(),
                            &"/cellblock/piptest".to_owned()));
    }

    #[test]
    fn test_unbind_roots() {
        let rule = String::from("local: /usr/lib mountpoint: /usr/lib opts: ro");
        assert!(unbind_mounts(&[rule].to_vec(),
                            &"/cellblock/piptest".to_owned()));

    }
}
