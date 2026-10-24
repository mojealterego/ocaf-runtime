#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid},
    macros::{map, tracepoint},
    maps::Array,
    programs::TracePointContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OpenatCount {
    pub target_tgid: u32,
    pub reserved: u32,
    pub count: u64,
    pub pid_tgid: u64,
}

#[map]
static OPENAT_COUNTS: Array<OpenatCount> = Array::with_max_entries(1, 0);

#[tracepoint]
pub fn ocaf_guest_openat(_ctx: TracePointContext) -> u32 {
    // The tracepoint itself selects sys_enter_openat. A target TGID is
    // populated by the guest loader before the Wasmtime child is released.
    match unsafe { observe_openat() } {
        Ok(()) => 0,
        Err(()) => 0,
    }
}

unsafe fn observe_openat() -> Result<(), ()> {
    let comm = bpf_get_current_comm().map_err(|_| ())?;
    if &comm[..8] != b"wasmtime" || comm[8] != 0 {
        return Ok(());
    }
    let pid_tgid = bpf_get_current_pid_tgid();
    let tgid = (pid_tgid >> 32) as u32;
    let record = OPENAT_COUNTS.get_ptr_mut(0).ok_or(())?;
    let record = unsafe { &mut *record };
    if record.target_tgid != tgid {
        return Ok(());
    }
    record.count = record.count.saturating_add(1);
    record.pid_tgid = pid_tgid;
    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
