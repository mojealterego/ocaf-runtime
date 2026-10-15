/* OCAF 0.39: kernel TC ingress program on a SINGLE ephemeral guest TAP.
 * DENY-ALL on the selected TAP only; never attach to physical interfaces.
 * Return TC_ACT_SHOT=2; this is TC eBPF, NOT Aya CgroupSkb.
 * Filter is loaded, attached and detached via trusted tc userspace in CI.
 */
#define SEC(x) __attribute__((section(x), used))
#define TC_ACT_SHOT 2
SEC("classifier")
int ocaf039_guest_tap_deny(void *ctx) {
    (void)ctx;
    return TC_ACT_SHOT;
}
char _license[] SEC("license") = "GPL";
