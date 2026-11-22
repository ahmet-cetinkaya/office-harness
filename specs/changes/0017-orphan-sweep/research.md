# 0017 research

## R1: Which crate reads a process identity on Linux, macOS, and Windows?

**Question.** The sweep must read, for a process id, the start time, the command line, and the machine's boot time on three OS, and decide whether a stored identity still names the same process.

**Sources.** `sysinfo` on crates.io (versions API) and its repository at 0.40.x: `Cargo.toml`, `CHANGELOG.md`, `src/common/system.rs`, `src/unix/linux/process.rs`, `src/unix/linux/system.rs`, `src/unix/apple/macos/process.rs`, `src/windows/process.rs`. Read on 2026-11-23.

**Findings.**

- Version 0.40.1 was released on 2026-10-30; it is the newest release at least two weeks old on 2026-11-23. License MIT. MSRV 1.95, below the project toolchain 1.100.0.
- `default-features = false, features = ["system"]` reads processes only. It adds `libc` and `memchr` on Linux, `objc2-core-foundation` and `objc2-io-kit` on macOS, and `windows` 0.62 and `ntapi` 0.4 on Windows.
- `refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, kind)` refreshes only the listed ids and removes dead ones.
- Start time:
  - Linux: `start_time()` is ticks divided by the clock rate (floor) plus the boot time from `/proc/stat`; `start_time_raw()` keeps the exact ticks after boot.
  - macOS: `pbi_start_tvsec`, whole seconds.
  - Windows: the `GetProcessTimes` creation time, converted to whole seconds; 0 when no handle opens.
- Boot time: seconds since the epoch, read again on each call. On Linux it comes from `btime`, which the kernel derives from the clock, so two reads can differ by a second.
- Command line: Linux `/proc/<pid>/cmdline`; macOS `sysctl KERN_PROCARGS2`; Windows the process environment block through `NtQueryInformationProcess`. A failed read leaves it empty and is only logged.
- Zombies are listed with the zombie status and can have an empty command line.
- Ending: `kill_with(Signal::Term)` returns `None` on Windows (only a forced end through `taskkill /F`), and on Unix it does not report the OS error.

**Decision for this change.**

- Read with `sysinfo` 0.40.1; end with `nix` on Unix and `windows-sys` on Windows (PLAN D1, D6).
- Linux stores `start_time_raw` (exact). macOS and Windows store whole seconds.
- Boot times match within 2 seconds.
- An empty command line is "not readable" and never matches; a zombie is "not running" (PLAN D2).

**Accepted risk.** On macOS and Windows a process id that is reused within the same second by a process with the same command line, in the same machine run, would match. Process ids on these systems are not reused that fast in practice. The office started the stored process, and the new one has the same command, so the case needs two runs of the same CLI command within one second and an id collision. T1 tests the case "same command, later start" with a start more than one second later.

**Not verified here.** Reading the command line of a same-user process on macOS and Windows in CI. T1 proves it in CI before any other task builds on it (PLAN D1).
