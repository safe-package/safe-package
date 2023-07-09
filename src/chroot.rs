use std::os::unix::fs;
use nix::unistd;
use subprocess;

pub fn bind_mounts(bind_mount_rules: &Vec<String>) -> bool {

    match subprocess::Exec::shell("sudo extras/bindmount-pip3.sh").join() {

        Ok(subprocess::ExitStatus::Exited(0)) => true,
        _ => false,
    }
}

pub fn unbind_mounts(bind_mount_rules: &Vec<String>) -> bool {

    match subprocess::Exec::shell("sudo extras/bindumount-pip3.sh").join() {

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
