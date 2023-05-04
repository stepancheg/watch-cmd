use std::env;
use std::fs;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::Duration;

use anyhow::Context;

fn now() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

fn capture_command(script: &str) -> anyhow::Result<String> {
    let mut command = Command::new("sh");
    command
        .args(["-ec", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let output = command.spawn()?.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("command failed");
    }
    let stdout = output.stdout;
    Ok(String::from_utf8(stdout)?)
}

/// Invoke `diff` to compare the two files.
fn diff(old: &str, new: &str) -> anyhow::Result<()> {
    let mut command = Command::new("diff");
    command.args(["-u", old, new, "-L", "old", "-L", "new"]);
    command.stdin(Stdio::null());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    let status = command
        .spawn()?
        .wait()?
        .code()
        .context("other that exit code")?;
    if status != 1 {
        anyhow::bail!("diff exited with status {}", status);
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (key, cmd) = match args.as_slice() {
        [key, cmd] => (key, cmd),
        _ => anyhow::bail!("Usage: {} <key> <cmd>", env::args().next().unwrap()),
    };

    let home = dirs::home_dir().context("no home dir")?;

    let dir = format!("{}/.watch-cmd/{}", home.display(), key);

    fs::create_dir_all(&dir)?;

    let mut out = capture_command(cmd)?;
    let mut out_path = format!("{}/{}", dir, now());
    fs::write(&out_path, &out)?;
    loop {
        thread::sleep(Duration::from_secs(1));
        let new = capture_command(cmd)?;
        let now = now();
        if new != out {
            eprintln!("changed at {now}");
            let new_path = format!("{}/{}", dir, now);
            fs::write(&new_path, &new)?;
            diff(&out_path, &new_path)?;
            out = new;
            out_path = new_path;
        }
    }
}
