// SPDX-License-Identifier: AGPL-3.0-or-later

use nix::sys::resource::{Resource, getrlimit};

use super::*;

#[test]
fn a_core_dump_of_the_server_has_size_0_and_on_linux_the_process_is_not_dumpable() {
    refuse_memory_dumps().expect("hardening");

    let (soft, hard) = getrlimit(Resource::RLIMIT_CORE).expect("limit");

    assert_eq!((soft, hard), (0, 0));
    #[cfg(target_os = "linux")]
    assert!(!nix::sys::prctl::get_dumpable().expect("dumpable"));
}
