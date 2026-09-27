//! Pinentry server speaking the Assuan protocol on stdin/stdout.
//!
//! Invoked by `rbw` when configured with:
//!   rbw config set pinentry "rofi-rbw-rs --pinentry"
//!
//! Mirrors `rbw-pinentry-keyring` from upstream rbw.

use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

fn rbw_profile() -> String {
    match std::env::var("RBW_PROFILE") {
        Ok(p) if !p.is_empty() => format!("rbw-{p}"),
        _ => "rbw".to_string(),
    }
}

fn read_command(line: &str) -> (&str, &str) {
    match line.find(' ') {
        Some(i) => (&line[..i], line[i + 1..].trim_end_matches('\r')),
        None => (line.trim_end_matches('\r'), ""),
    }
}

fn write_ok(out: &mut impl Write) -> io::Result<()> {
    out.write_all(b"OK\n")?;
    out.flush()
}

fn write_err(out: &mut impl Write, msg: &str) -> io::Result<()> {
    writeln!(out, "ERR {msg}")?;
    out.flush()
}

fn write_data(out: &mut impl Write, data: &str) -> io::Result<()> {
    // Assuan data-line escaping: only % and \n are strictly required.
    let escaped = data.replace('%', "%25").replace('\n', "%0A");
    writeln!(out, "D {escaped}")?;
    out.flush()
}

fn lookup_keyring(profile: &str) -> Result<Option<String>> {
    let out = Command::new("secret-tool")
        .args([
            "lookup",
            "application", "rbw",
            "profile", profile,
            "type", "master_password",
        ])
        .output()
        .context("failed to run `secret-tool` — is libsecret installed?")?;

    match out.status.code() {
        Some(0) => Ok(Some(
            String::from_utf8(out.stdout).context("secret-tool returned non-UTF-8")?,
        )),
        Some(1) => Ok(None),
        _ => bail!(
            "`secret-tool lookup` failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ),
    }
}

fn store_keyring(profile: &str, password: &str) -> Result<()> {
    let label = format!("{profile} master password");
    let mut child = Command::new("secret-tool")
        .args([
            "store",
            "--label", &label,
            "application", "rbw",
            "profile", profile,
            "type", "master_password",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run `secret-tool store`")?;

    {
        let mut stdin = child.stdin.take().expect("stdin piped");
        // No trailing newline — secret-tool stores stdin verbatim.
        stdin.write_all(password.as_bytes())?;
    }

    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "`secret-tool store` failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn clear_keyring(profile: &str) -> Result<()> {
    let status = Command::new("secret-tool")
        .args([
            "clear",
            "application", "rbw",
            "profile", profile,
            "type", "master_password",
        ])
        .status()
        .context("failed to run `secret-tool clear`")?;
    if !status.success() {
        bail!("`secret-tool clear` exited with {status}");
    }
    Ok(())
}

/// Shell out to the real pinentry to collect a secret interactively.
fn ask_pinentry(title: &str, prompt: &str, desc: &str) -> Result<String> {
    let mut child = Command::new("pinentry")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn `pinentry` — is it installed?")?;

    {
        let mut stdin = child.stdin.take().expect("stdin piped");
        writeln!(stdin, "SETTITLE {title}")?;
        writeln!(stdin, "SETPROMPT {prompt}")?;
        writeln!(stdin, "SETDESC {desc}")?;
        writeln!(stdin, "GETPIN")?;
    }

    let out = child.wait_with_output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("D ") {
            return Ok(rest.replace("%0A", "\n").replace("%25", "%"));
        }
    }
    bail!("pinentry did not return a D line")
}

pub fn run() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut title = String::new();
    let mut desc = String::new();
    let mut prompt = String::new();
    let profile = rbw_profile();

    // pinentry greeting.
    write_ok(&mut stdout)?;

    for line in stdin.lock().lines() {
        let line = line.context("read from stdin")?;
        let (cmd, args) = read_command(&line);

        match cmd {
            "SETTITLE" => { title = args.to_string(); write_ok(&mut stdout)?; }
            "SETDESC"  => { desc  = args.to_string(); write_ok(&mut stdout)?; }
            "SETPROMPT"=> { prompt = args.to_string(); write_ok(&mut stdout)?; }
            "SETOK" | "SETCANCEL" | "SETNOTOK" | "SETERROR" | "SETREPEAT"
            | "SETREPEATERROR" | "SETREPEATOK" | "SETREPEATPROMPT"
            | "OPTION" | "SETKEYINFO" | "SETGENKEY" | "SETQUALITYBAR"
            | "SETQUALITYBAR_TT" | "SETDATAFILE" | "SETHIDDEN" => {
                write_ok(&mut stdout)?;
            }
            "GETPIN" => {
                if prompt != "Master Password" {
                    // Non-master prompts (2FA, etc.) go straight through.
                    let value = ask_pinentry(&title, &prompt, &desc)?;
                    write_data(&mut stdout, &value)?;
                    write_ok(&mut stdout)?;
                    continue;
                }

                match lookup_keyring(&profile) {
                    Ok(Some(v)) => {
                        write_data(&mut stdout, &v)?;
                        write_ok(&mut stdout)?;
                    }
                    Ok(None) => {
                        // First run: prompt, store, return.
                        let v = ask_pinentry(&title, &prompt, &desc)?;
                        if !v.is_empty() {
                            if let Err(e) = store_keyring(&profile, &v) {
                                eprintln!(
                                    "rofi-rbw-rs: could not store to keyring: {e:#}"
                                );
                            }
                        }
                        write_data(&mut stdout, &v)?;
                        write_ok(&mut stdout)?;
                    }
                    Err(e) => {
                        eprintln!("rofi-rbw-rs: keyring lookup failed: {e:#}");
                        let v = ask_pinentry(&title, &prompt, &desc)?;
                        write_data(&mut stdout, &v)?;
                        write_ok(&mut stdout)?;
                    }
                }
            }
            "BYE" => return Ok(()),
            "" => continue,
            _ => write_err(&mut stdout, "Unknown command")?,
        }
    }
    Ok(())
}

pub fn clear_stored() -> Result<()> {
    clear_keyring(&rbw_profile())
}
