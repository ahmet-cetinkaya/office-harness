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
- Boot time: seconds since the epoch, read again on each call. On Linux it comes from `btime`, which the kernel derives from the clock, so it moves with clock steps (a large time correction can move it by much more than a second). On Windows it comes from the tick count and moves the same way. Linux also has `/proc/sys/kernel/random/boot_id`, a value that is fixed for one boot and changes at the next one.
- Command line: Linux `/proc/<pid>/cmdline`; macOS `sysctl KERN_PROCARGS2`; Windows the process environment block through `NtQueryInformationProcess`. A failed read leaves it empty and is only logged.
- Zombies are listed with the zombie status and can have an empty command line.
- Ending: `kill_with(Signal::Term)` returns `None` on Windows (only a forced end through `taskkill /F`), and on Unix it does not report the OS error. All of it works by process id, so it has the id reuse race between the read and the end.
- Process handles (kernel documentation and the Microsoft API reference, not tested here): Linux 5.3 and later has `pidfd_open` and `pidfd_send_signal`; a pidfd refers to one process and never to a later process with the same id, and it is readable (poll) when the process has exited. Windows `OpenProcess` gives a handle that keeps referring to that process; `GetProcessTimes` gives the creation time in 100 ns units; `NtQueryInformationProcess` with `ProcessCommandLineInformation` (class 60, Windows 8.1 and later) gives the command line on that handle; `TerminateProcess` ends it on that handle. macOS has no such handle.

**Decision for this change.**

- Use `sysinfo` 0.40.1 to list processes and parents, and to read the identity on Linux and macOS. Use handles to read and end where the OS has them: pidfd on Linux (through `libc`), `windows-sys` on Windows; `nix` `kill(pid)` on macOS and on Linux before 5.3 (PLAN D1, D6).
- Linux stores `start_time_raw` (exact). Windows stores the 100 ns creation time. macOS stores whole seconds.
- The machine run is `boot_id` on Linux (exact). On macOS and Windows boot times match within 2 seconds, and a larger difference with all else equal gives a warning, not a clean result (PLAN D2).
- An empty command line is "not readable" and never matches; a zombie is "not running" (PLAN D2).

**Accepted risk (macOS, and Linux before 5.3).** There is no process handle. The sweep reads again and then ends by process id. A process that exits in that short time, whose id is reused by a process with the same start second and the same command line, would match and be ended. The office started the stored process, and the new one needs the same command and an id collision within the same second. Windows and Linux 5.3 and later do not have this risk, because the end goes through the handle that was checked. T1 tests the case "same command, later start" with a start more than one second later, and tests the old-kernel path.

**Not verified here.** Reading the command line of a same-user process on macOS and Windows in CI; the feature names of `windows-sys` for `NtQueryInformationProcess`; a `libc` direct dependency for the pidfd calls. T1 proves it in CI before any other task builds on it (PLAN D1).
