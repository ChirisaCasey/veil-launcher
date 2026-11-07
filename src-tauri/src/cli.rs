//! Command-line parsing (no Tauri involved).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Normal tray app.
    App,
    /// Elevated one-shot file op; exit code = result.
    Helper { op: String, args: String },
    /// Uninstaller cleanup: restore original, remove startup entries.
    Cleanup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub mode: Mode,
    /// Start hidden with the watcher on (autostart / scheduled task).
    pub background: bool,
    pub minimized: bool,
}

pub fn parse<I, S>(args: I) -> Args
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let v: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    let mut out = Args {
        mode: Mode::App,
        background: false,
        minimized: false,
    };
    let mut i = 0;
    while i < v.len() {
        match v[i].as_str() {
            "--background" => out.background = true,
            "--minimized" => out.minimized = true,
            "--cleanup" => out.mode = Mode::Cleanup,
            "--helper" => {
                let op = v.get(i + 1).cloned().unwrap_or_default();
                let args = v.get(i + 2).cloned().unwrap_or_default();
                out.mode = Mode::Helper { op, args };
                i += 2;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modes() {
        assert_eq!(parse(Vec::<String>::new()).mode, Mode::App);
        let a = parse(["--background"]);
        assert!(a.background && !a.minimized);
        assert!(parse(["--minimized"]).minimized);
        assert_eq!(parse(["--cleanup"]).mode, Mode::Cleanup);
        assert_eq!(
            parse(["--helper", "swap", "7b7d"]).mode,
            Mode::Helper {
                op: "swap".into(),
                args: "7b7d".into()
            }
        );
        assert_eq!(
            parse(["--helper"]).mode,
            Mode::Helper {
                op: String::new(),
                args: String::new()
            }
        );
    }
}
