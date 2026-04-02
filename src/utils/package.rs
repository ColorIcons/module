use std::{collections::HashMap, process::Command};

pub fn get_installed_packages() -> HashMap<String, String> {
    let output = Command::new("pm")
        .args(["list", "packages", "-3", "-f"])
        .output()
        .expect("无法执行 pm list packages");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();
    for line in stdout.lines() {
        if let Some((apk_part, pkg_name)) = line.rsplit_once('=')
            && let Some(apk_path) = apk_part.strip_prefix("package:")
        {
            map.insert(pkg_name.to_string(), apk_path.to_string());
        }
    }
    map
}
