//! Elevation helpers: token check, `runas` helper process, scheduled task, hidden child processes.

use anyhow::{Context, Result, bail};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

pub const TASK_NAME: &str = "VeilLauncher";

/// A `Command` that never flashes a console window.
pub fn hidden_command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elev = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elev as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        CloseHandle(token);
        ok != 0 && elev.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// JSON args are hex-encoded on the command line so quoting can never break them.
pub fn encode_args(v: &serde_json::Value) -> String {
    hex::encode(v.to_string())
}

pub fn decode_args(s: &str) -> Result<serde_json::Value> {
    let t = s.trim();
    let text = if t.starts_with('{') {
        t.to_string()
    } else {
        String::from_utf8(hex::decode(t).context("helper args are not hex")?)?
    };
    serde_json::from_str(&text).context("helper args are not JSON")
}

pub const HELPER_OK: i32 = 0;
pub const HELPER_FAILED: i32 = 1;
pub const HELPER_USAGE: i32 = 2;
pub const HELPER_VERIFY_FAILED: i32 = 3;

/// Re-launch this exe elevated (`runas`) as `--helper <op> <args>`, wait, return its exit code.
#[cfg(windows)]
pub fn run_elevated_helper(op: &str, args: &serde_json::Value) -> Result<i32> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_CANCELLED, GetLastError, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        ShellExecuteExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = std::env::current_exe().context("locating own exe")?;
    let wide = |s: &OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let file = wide(exe.as_os_str());
    let params = wide(OsStr::new(&format!("--helper {op} {}", encode_args(args))));
    let verb = wide(OsStr::new("runas"));
    log::info!("requesting elevation for helper op {op}");
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = params.as_ptr();
        info.nShow = SW_HIDE;
        if ShellExecuteExW(&mut info) == 0 {
            let err = GetLastError();
            if err == ERROR_CANCELLED {
                bail!("Administrator permission was declined");
            }
            bail!("could not start elevated helper (error {err})");
        }
        if info.hProcess.is_null() {
            bail!("elevated helper did not return a process handle");
        }
        let wait = WaitForSingleObject(info.hProcess, 5 * 60 * 1000);
        let mut code: u32 = HELPER_FAILED as u32;
        if wait == WAIT_OBJECT_0 {
            GetExitCodeProcess(info.hProcess, &mut code);
        }
        CloseHandle(info.hProcess);
        if wait != WAIT_OBJECT_0 {
            bail!("elevated helper timed out");
        }
        Ok(code as i32)
    }
}

#[cfg(not(windows))]
pub fn run_elevated_helper(_op: &str, _args: &serde_json::Value) -> Result<i32> {
    bail!("elevation is only supported on Windows")
}

/// Run `op` through the elevated helper and turn its exit code into a Result.
pub fn helper_or_err(op: &str, args: &serde_json::Value) -> Result<()> {
    match run_elevated_helper(op, args)? {
        HELPER_OK => Ok(()),
        HELPER_VERIFY_FAILED => bail!("elevated {op}: verification failed"),
        code => bail!("elevated {op} failed (exit code {code})"),
    }
}

fn schtasks(args: &[&str]) -> Result<std::process::Output> {
    hidden_command("schtasks")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("running schtasks")
}

/// The `/TR` value: `"<exe>" --background`.
pub fn task_command_line(exe: &Path) -> String {
    format!("\"{}\" --background", exe.display())
}

pub fn task_exists() -> bool {
    if !cfg!(windows) {
        return false;
    }
    schtasks(&["/Query", "/TN", TASK_NAME])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Must already be elevated.
pub fn create_task_direct(exe: &Path) -> Result<()> {
    let tr = task_command_line(exe);
    let out = schtasks(&[
        "/Create", "/TN", TASK_NAME, "/TR", &tr, "/SC", "ONLOGON", "/RL", "HIGHEST", "/F",
    ])?;
    if !out.status.success() {
        bail!(
            "schtasks /Create failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Must already be elevated (for HIGHEST tasks).
pub fn delete_task_direct() -> Result<()> {
    let out = schtasks(&["/Delete", "/TN", TASK_NAME, "/F"])?;
    if !out.status.success() {
        bail!(
            "schtasks /Delete failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// Create the logon task, prompting for elevation if needed.
pub fn install_task() -> Result<()> {
    let exe = std::env::current_exe()?;
    if is_elevated() {
        create_task_direct(&exe)
    } else {
        helper_or_err(
            "install-task",
            &serde_json::json!({ "exe": exe.to_string_lossy() }),
        )
    }
}

/// Remove the logon task, prompting for elevation if needed. No-op if absent.
pub fn remove_task() -> Result<()> {
    if !task_exists() {
        return Ok(());
    }
    if is_elevated() {
        delete_task_direct()
    } else {
        helper_or_err("remove-task", &serde_json::json!({}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_roundtrip() {
        let v =
            serde_json::json!({"source": "C:\\a b\\\"x\".mp4", "target": "D:/Riot Games/x.mp4"});
        let enc = encode_args(&v);
        assert!(enc.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(decode_args(&enc).unwrap(), v);
        assert_eq!(
            decode_args(r#"{"a":1}"#).unwrap(),
            serde_json::json!({"a":1})
        );
        assert!(decode_args("zz").is_err());
    }

    #[test]
    fn task_line_quotes_exe() {
        let l = task_command_line(Path::new(
            r"C:\Program Files\Veil Launcher\veil-launcher.exe",
        ));
        assert_eq!(
            l,
            r#""C:\Program Files\Veil Launcher\veil-launcher.exe" --background"#
        );
    }

    #[test]
    fn elevation_check_does_not_panic() {
        let _ = is_elevated();
    }
}
