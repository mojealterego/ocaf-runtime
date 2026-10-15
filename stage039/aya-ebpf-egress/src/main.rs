#![no_std]
#![no_main]

// Defense-in-depth for a private, operator-identified microVM cgroup.
// BPF_CGROUP_INET_EGRESS: return 0 means the kernel drops the packet.
// This source alone proves nothing until verifier LOAD + ATTACH and a
// negative packet test are recorded on a real BPF-enabled Linux host.
use aya_ebpf::{macros::cgroup_skb, programs::SkBuffContext};

#[cgroup_skb(egress)]
pub fn ocaf_deny_egress(_ctx: SkBuffContext) -> i32 {
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
