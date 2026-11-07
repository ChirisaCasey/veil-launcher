//! Process watcher: a pure state machine fed by a `ProcessSource`.
//!
//! idle -> riotClient -> waiting (game seen) -> [trigger + delay] -> Swap -> swapped
//!      -> game exits (debounced) -> Restore -> restoring -> idle

use serde::Serialize;
use std::time::Duration;

use crate::config::SwapTrigger;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    #[default]
    Idle,
    RiotClient,
    Waiting,
    Swapped,
    Restoring,
    Error,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Idle => "Idle",
            Phase::RiotClient => "Riot Client open",
            Phase::Waiting => "Waiting for VALORANT",
            Phase::Swapped => "Custom background active",
            Phase::Restoring => "Restoring original",
            Phase::Error => "Error",
        }
    }
}

/// Riot Client UI processes (NOT RiotClientServices.exe, which lingers in the background).
pub const RIOT_CLIENT_UI: [&str; 3] = [
    "riot client.exe",
    "riotclientux.exe",
    "riotclientuxrender.exe",
];
/// The game itself and its launcher stub.
pub const GAME: [&str; 2] = ["valorant-win64-shipping.exe", "valorant.exe"];

pub const POLL_INTERVAL: Duration = Duration::from_millis(1500);
/// Consecutive polls without the game before we consider it closed.
pub const GONE_TICKS: u32 = 2;
pub const RESTORE_RETRY: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProcSnapshot {
    pub riot_client_ui: bool,
    pub game: bool,
}

pub fn classify<'a>(names: impl IntoIterator<Item = &'a str>) -> ProcSnapshot {
    let mut s = ProcSnapshot::default();
    for n in names {
        let l = n.to_ascii_lowercase();
        if RIOT_CLIENT_UI.contains(&l.as_str()) {
            s.riot_client_ui = true;
        }
        if GAME.contains(&l.as_str()) {
            s.game = true;
        }
    }
    s
}

pub trait ProcessSource: Send {
    fn snapshot(&mut self) -> ProcSnapshot;
}

pub struct SysinfoSource {
    sys: sysinfo::System,
}

impl SysinfoSource {
    pub fn new() -> Self {
        Self {
            sys: sysinfo::System::new(),
        }
    }
}

impl Default for SysinfoSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessSource for SysinfoSource {
    fn snapshot(&mut self) -> ProcSnapshot {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        let names: Vec<String> = self
            .sys
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_string())
            .collect();
        classify(names.iter().map(String::as_str))
    }
}

/// Scripted source for tests / simulations.
pub struct FakeSource {
    pub script: std::collections::VecDeque<ProcSnapshot>,
    pub last: ProcSnapshot,
}

impl FakeSource {
    pub fn new(script: impl IntoIterator<Item = ProcSnapshot>) -> Self {
        Self {
            script: script.into_iter().collect(),
            last: ProcSnapshot::default(),
        }
    }
}

impl ProcessSource for FakeSource {
    fn snapshot(&mut self) -> ProcSnapshot {
        if let Some(s) = self.script.pop_front() {
            self.last = s;
        }
        self.last
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchConfig {
    pub trigger: SwapTrigger,
    pub delay: Duration,
    /// enabled && a video is selected && a target exists
    pub armed: bool,
    /// Ground truth from the journal: the custom video is in the game folder.
    pub swapped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Swap,
    Restore,
}

#[derive(Debug, Clone, Default)]
pub struct Machine {
    pub phase: Phase,
    trigger_at: Option<Duration>,
    absent_ticks: u32,
    /// Manual restore during this game session: don't re-swap until the game restarts.
    suppressed: bool,
    swap_failed: bool,
    restore_retry_at: Option<Duration>,
    /// At least one step has run (distinguishes "app started mid-game" from "game launched").
    started: bool,
}

impl Machine {
    pub fn new() -> Self {
        Self::default()
    }

    fn reset_session(&mut self) {
        self.trigger_at = None;
        self.suppressed = false;
        self.swap_failed = false;
        self.restore_retry_at = None;
    }

    fn rest_phase(o: ProcSnapshot) -> Phase {
        if o.riot_client_ui {
            Phase::RiotClient
        } else {
            Phase::Idle
        }
    }

    /// Advance one poll. `now` is a monotonic timestamp.
    pub fn step(&mut self, o: ProcSnapshot, now: Duration, c: WatchConfig) -> Action {
        if o.game {
            self.absent_ticks = 0;
        } else {
            self.absent_ticks = self.absent_ticks.saturating_add(1);
        }
        let game_gone = !o.game && self.absent_ticks >= GONE_TICKS;
        let first_step = !self.started;
        self.started = true;

        match self.phase {
            Phase::Swapped => {
                if game_gone {
                    self.phase = Phase::Restoring;
                    return Action::Restore;
                }
                if !c.swapped {
                    // Restored behind our back (manual restore / settings change).
                    self.phase = if o.game {
                        Phase::Waiting
                    } else {
                        Self::rest_phase(o)
                    };
                    self.suppressed = o.game;
                }
                Action::None
            }
            // Waiting for the engine to report the restore result.
            Phase::Restoring => Action::None,
            Phase::Error => {
                if c.swapped {
                    if o.game {
                        // Game (re)started while a swap is in place: keep it, restore on exit.
                        self.phase = Phase::Swapped;
                        return Action::None;
                    }
                    if game_gone && self.restore_retry_at.is_none_or(|t| now >= t) {
                        self.phase = Phase::Restoring;
                        return Action::Restore;
                    }
                    return Action::None;
                }
                if game_gone {
                    self.reset_session();
                    self.phase = Self::rest_phase(o);
                }
                Action::None
            }
            Phase::Idle | Phase::RiotClient | Phase::Waiting => {
                if o.game {
                    if c.swapped && first_step {
                        // App (re)started while the game runs with our video in place: resume.
                        // On a fresh launch we go through the trigger instead, because Riot's
                        // file check may have repaired the file (the swap is idempotent).
                        self.phase = Phase::Swapped;
                        return Action::None;
                    }
                    if self.phase != Phase::Waiting {
                        self.phase = Phase::Waiting;
                        self.trigger_at = None;
                    }
                    if self.suppressed || self.swap_failed || !c.armed {
                        self.trigger_at = None;
                        return Action::None;
                    }
                    let ready = match c.trigger {
                        SwapTrigger::GameStarted => true,
                        SwapTrigger::RiotClientClosed => !o.riot_client_ui,
                    };
                    if !ready {
                        self.trigger_at = None;
                        return Action::None;
                    }
                    let at = *self.trigger_at.get_or_insert(now + c.delay);
                    if now >= at {
                        Action::Swap
                    } else {
                        Action::None
                    }
                } else {
                    if self.phase == Phase::Waiting && !game_gone {
                        return Action::None; // debounce launcher stub -> shipping handover
                    }
                    if self.phase == Phase::Waiting && c.swapped {
                        // Session ended with our video still in place (e.g. disabled mid-game).
                        self.phase = Phase::Restoring;
                        return Action::Restore;
                    }
                    if self.phase == Phase::Waiting || self.phase != Self::rest_phase(o) {
                        self.reset_session();
                    }
                    self.phase = Self::rest_phase(o);
                    Action::None
                }
            }
        }
    }

    pub fn on_swap_result(&mut self, ok: bool) {
        if ok {
            self.phase = Phase::Swapped;
            self.swap_failed = false;
        } else {
            self.phase = Phase::Error;
            self.swap_failed = true;
        }
        self.trigger_at = None;
    }

    pub fn on_restore_result(&mut self, ok: bool, now: Duration, o: ProcSnapshot) {
        if ok {
            self.reset_session();
            self.phase = Self::rest_phase(o);
        } else {
            self.phase = Phase::Error;
            self.restore_retry_at = Some(now + RESTORE_RETRY);
        }
    }

    /// The user restored manually. While the game runs, don't swap again this session.
    pub fn on_manual_restore(&mut self, game_running: bool, o: ProcSnapshot) {
        self.trigger_at = None;
        if game_running {
            self.suppressed = true;
            self.phase = Phase::Waiting;
        } else {
            self.reset_session();
            self.phase = Self::rest_phase(o);
        }
    }

    /// The user applied manually.
    pub fn on_manual_swap(&mut self, game_running: bool) {
        self.suppressed = false;
        self.swap_failed = false;
        self.trigger_at = None;
        if game_running {
            self.phase = Phase::Swapped;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RC: ProcSnapshot = ProcSnapshot {
        riot_client_ui: true,
        game: false,
    };
    const RC_GAME: ProcSnapshot = ProcSnapshot {
        riot_client_ui: true,
        game: true,
    };
    const GAME_ONLY: ProcSnapshot = ProcSnapshot {
        riot_client_ui: false,
        game: true,
    };
    const NONE: ProcSnapshot = ProcSnapshot {
        riot_client_ui: false,
        game: false,
    };

    fn cfg(trigger: SwapTrigger, delay: u64) -> WatchConfig {
        WatchConfig {
            trigger,
            delay: Duration::from_secs(delay),
            armed: true,
            swapped: false,
        }
    }

    /// Drive a machine with a fake source, simulating a perfect engine (swap/restore succeed).
    /// Returns (time, action, phase-after) for every tick that produced an action.
    fn simulate(script: Vec<ProcSnapshot>, mut c: WatchConfig) -> (Vec<(u64, Action)>, Vec<Phase>) {
        let mut src = FakeSource::new(script.clone());
        let mut m = Machine::new();
        let mut actions = Vec::new();
        let mut phases = Vec::new();
        for i in 0..script.len() as u64 {
            let now = Duration::from_secs(i);
            let o = src.snapshot();
            let a = m.step(o, now, c);
            match a {
                Action::Swap => {
                    c.swapped = true;
                    m.on_swap_result(true);
                }
                Action::Restore => {
                    c.swapped = false;
                    m.on_restore_result(true, now, o);
                }
                Action::None => {}
            }
            if a != Action::None {
                actions.push((i, a));
            }
            phases.push(m.phase);
        }
        (actions, phases)
    }

    #[test]
    fn classify_names() {
        let s = classify(["explorer.exe", "Riot Client.exe", "RiotClientServices.exe"]);
        assert_eq!(s, RC);
        let s = classify(["RiotClientServices.exe", "VALORANT-Win64-Shipping.exe"]);
        assert_eq!(s, GAME_ONLY);
        assert_eq!(classify(["RiotClientServices.exe"]), NONE);
        assert!(classify(["RIOTCLIENTUX.EXE"]).riot_client_ui);
    }

    #[test]
    fn riot_client_closed_trigger_with_delay() {
        // t0 idle, t1-2 RC, t3-4 RC+game (verification), t5.. game only, ..., game exits at t12.
        let script = vec![
            NONE, RC, RC, RC_GAME, RC_GAME, GAME_ONLY, GAME_ONLY, GAME_ONLY, GAME_ONLY, GAME_ONLY,
            GAME_ONLY, GAME_ONLY, NONE, NONE, NONE,
        ];
        let (actions, phases) = simulate(script, cfg(SwapTrigger::RiotClientClosed, 2));
        // Trigger condition first true at t5 -> swap at t7.
        assert_eq!(actions, vec![(7, Action::Swap), (13, Action::Restore)]);
        assert_eq!(phases[0], Phase::Idle);
        assert_eq!(phases[1], Phase::RiotClient);
        assert_eq!(phases[3], Phase::Waiting);
        assert_eq!(phases[6], Phase::Waiting);
        assert_eq!(phases[7], Phase::Swapped);
        assert_eq!(phases[12], Phase::Swapped, "one missing poll is debounced");
        assert_eq!(phases[13], Phase::Idle);
    }

    #[test]
    fn riot_client_reappearing_resets_delay() {
        let script = vec![
            RC_GAME, GAME_ONLY, RC_GAME, GAME_ONLY, GAME_ONLY, GAME_ONLY, GAME_ONLY,
        ];
        let (actions, _) = simulate(script, cfg(SwapTrigger::RiotClientClosed, 2));
        // Ready at t1 (fires t3) but RC came back at t2; ready again t3 -> fires t5.
        assert_eq!(actions, vec![(5, Action::Swap)]);
    }

    #[test]
    fn game_started_trigger_ignores_riot_client() {
        let script = vec![RC, RC_GAME, RC_GAME, RC_GAME, RC_GAME];
        let (actions, _) = simulate(script.clone(), cfg(SwapTrigger::GameStarted, 0));
        assert_eq!(actions, vec![(1, Action::Swap)]);
        let (actions, _) = simulate(script, cfg(SwapTrigger::GameStarted, 3));
        assert_eq!(actions, vec![(4, Action::Swap)]);
    }

    #[test]
    fn riot_client_closed_never_fires_while_client_open() {
        let script = vec![RC_GAME; 10];
        let (actions, phases) = simulate(script, cfg(SwapTrigger::RiotClientClosed, 0));
        assert!(actions.is_empty());
        assert_eq!(*phases.last().unwrap(), Phase::Waiting);
    }

    #[test]
    fn not_armed_never_swaps() {
        let mut c = cfg(SwapTrigger::GameStarted, 0);
        c.armed = false;
        let (actions, _) = simulate(vec![GAME_ONLY; 5], c);
        assert!(actions.is_empty());
    }

    #[test]
    fn game_quits_before_trigger_returns_to_idle() {
        let script = vec![RC_GAME, RC_GAME, RC, NONE, NONE, NONE];
        let (actions, phases) = simulate(script, cfg(SwapTrigger::RiotClientClosed, 0));
        assert!(actions.is_empty());
        assert_eq!(phases[2], Phase::Waiting);
        assert_eq!(phases[3], Phase::Idle);
    }

    #[test]
    fn failed_swap_goes_to_error_without_retry_loop() {
        let mut m = Machine::new();
        let c = cfg(SwapTrigger::GameStarted, 0);
        assert_eq!(m.step(GAME_ONLY, Duration::from_secs(0), c), Action::Swap);
        m.on_swap_result(false);
        assert_eq!(m.phase, Phase::Error);
        for i in 1..5 {
            assert_eq!(m.step(GAME_ONLY, Duration::from_secs(i), c), Action::None);
        }
        // Game exits -> back to idle, next session can try again.
        m.step(NONE, Duration::from_secs(6), c);
        m.step(NONE, Duration::from_secs(7), c);
        assert_eq!(m.phase, Phase::Idle);
        assert_eq!(m.step(GAME_ONLY, Duration::from_secs(8), c), Action::Swap);
    }

    #[test]
    fn failed_restore_is_retried() {
        let mut m = Machine::new();
        let mut c = cfg(SwapTrigger::GameStarted, 0);
        m.step(GAME_ONLY, Duration::from_secs(0), c);
        m.on_swap_result(true);
        c.swapped = true;
        m.step(NONE, Duration::from_secs(1), c);
        assert_eq!(m.step(NONE, Duration::from_secs(2), c), Action::Restore);
        m.on_restore_result(false, Duration::from_secs(2), NONE);
        assert_eq!(m.phase, Phase::Error);
        assert_eq!(m.step(NONE, Duration::from_secs(5), c), Action::None);
        assert_eq!(m.step(NONE, Duration::from_secs(17), c), Action::Restore);
        m.on_restore_result(true, Duration::from_secs(17), NONE);
        assert_eq!(m.phase, Phase::Idle);
    }

    #[test]
    fn startup_with_swap_and_game_running_resumes_swapped() {
        let mut m = Machine::new();
        let mut c = cfg(SwapTrigger::RiotClientClosed, 2);
        c.swapped = true;
        assert_eq!(m.step(GAME_ONLY, Duration::ZERO, c), Action::None);
        assert_eq!(m.phase, Phase::Swapped);
    }

    #[test]
    fn manual_restore_suppresses_until_game_restarts() {
        let mut m = Machine::new();
        let mut c = cfg(SwapTrigger::GameStarted, 0);
        assert_eq!(m.step(GAME_ONLY, Duration::from_secs(0), c), Action::Swap);
        m.on_swap_result(true);
        c.swapped = false; // user hit "Restore original now"
        m.on_manual_restore(true, GAME_ONLY);
        for i in 1..5 {
            assert_eq!(m.step(GAME_ONLY, Duration::from_secs(i), c), Action::None);
        }
        m.step(NONE, Duration::from_secs(5), c);
        m.step(NONE, Duration::from_secs(6), c);
        assert_eq!(m.phase, Phase::Idle);
        assert_eq!(m.step(GAME_ONLY, Duration::from_secs(7), c), Action::Swap);
    }

    #[test]
    fn swapped_but_disarmed_session_still_restores_on_exit() {
        let mut c = cfg(SwapTrigger::GameStarted, 0);
        c.swapped = true;
        c.armed = false;
        let script = vec![NONE, GAME_ONLY, GAME_ONLY, NONE, NONE];
        let (actions, phases) = simulate(script, c);
        assert_eq!(actions, vec![(4, Action::Restore)]);
        assert_eq!(phases[2], Phase::Waiting);
        assert_eq!(phases[4], Phase::Idle);
    }

    #[test]
    fn manual_apply_before_launch_is_reapplied_and_restored() {
        // Applied while the game is not running: watcher swaps (idempotently) after the
        // Riot Client check and restores on exit.
        let mut c = cfg(SwapTrigger::RiotClientClosed, 0);
        c.swapped = true;
        let script = vec![NONE, RC, RC_GAME, GAME_ONLY, GAME_ONLY, NONE, NONE];
        let (actions, phases) = simulate(script, c);
        assert_eq!(actions, vec![(3, Action::Swap), (6, Action::Restore)]);
        assert_eq!(phases[2], Phase::Waiting);
        assert_eq!(phases[3], Phase::Swapped);
    }
}
