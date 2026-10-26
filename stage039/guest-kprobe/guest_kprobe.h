/* Optional guest PID1 kprobe proof for the two-tenant WASI integration lab. */
#ifndef OCAF_STAGE039_GUEST_KPROBE_H
#define OCAF_STAGE039_GUEST_KPROBE_H
#include <linux/perf_event.h>

static int ocaf_kprobe_event_id(unsigned int *id) {
    char text[32] = {0};
    int fd = open("/sys/kernel/tracing/events/kprobes/ocaf039_getpid/id",
                  O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) return -1;
    ssize_t n = read(fd, text, sizeof(text) - 1);
    close(fd);
    return n > 0 && n < (ssize_t)sizeof(text) && sscanf(text, "%u", id) == 1
           && *id > 0 ? 0 : -1;
}

static int ocaf_guest_kprobe_proof(unsigned int *program_id, uint64_t *count) {
    unsigned char magic[2];
    int btf = open("/sys/kernel/btf/vmlinux", O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (btf < 0) return -1;
    int valid = read(btf, magic, sizeof(magic)) == (ssize_t)sizeof(magic)
                && magic[0] == 0x9f && magic[1] == 0xeb;
    close(btf);
    if (!valid) return -1;
    int control = open("/sys/kernel/tracing/kprobe_events",
                       O_WRONLY | O_CLOEXEC | O_NOFOLLOW);
    if (control < 0) return -1;
    static const char event[] = "p:ocaf039_getpid __x64_sys_getpid\n";
    valid = write(control, event, sizeof(event) - 1) == (ssize_t)(sizeof(event) - 1);
    close(control);
    if (!valid) return -1;
    unsigned int event_id = 0;
    if (ocaf_kprobe_event_id(&event_id) != 0) return -1;
    struct bpf_insn program[] = {
        {.code = BPF_ALU64 | BPF_MOV | BPF_K, .dst_reg = BPF_REG_0, .imm = 1},
        {.code = BPF_JMP | BPF_EXIT},
    };
    static const char license[] = "GPL";
    union bpf_attr attr = {0};
    attr.prog_type = BPF_PROG_TYPE_KPROBE;
    attr.insn_cnt = sizeof(program) / sizeof(program[0]);
    attr.insns = (uint64_t)(uintptr_t)program;
    attr.license = (uint64_t)(uintptr_t)license;
    int prog = (int)syscall(SYS_bpf, BPF_PROG_LOAD, &attr, sizeof(attr));
    if (prog < 0) return -1;
    struct bpf_prog_info info = {0};
    memset(&attr, 0, sizeof(attr));
    attr.info.bpf_fd = (uint32_t)prog;
    attr.info.info_len = sizeof(info);
    attr.info.info = (uint64_t)(uintptr_t)&info;
    if (syscall(SYS_bpf, BPF_OBJ_GET_INFO_BY_FD, &attr, sizeof(attr)) < 0
        || info.id == 0) { close(prog); return -1; }
    struct perf_event_attr perf = {0};
    perf.type = PERF_TYPE_TRACEPOINT;
    perf.size = sizeof(perf);
    perf.config = event_id;
    perf.sample_period = 1;
    perf.disabled = 1;
    int link = (int)syscall(SYS_perf_event_open, &perf, -1, 0, -1,
                            PERF_FLAG_FD_CLOEXEC);
    if (link < 0) { close(prog); return -1; }
    if (ioctl(link, PERF_EVENT_IOC_SET_BPF, prog) != 0
        || ioctl(link, PERF_EVENT_IOC_ENABLE, 0) != 0) {
        close(link); close(prog); return -1;
    }
    for (int i = 0; i < 64; i++) (void)syscall(SYS_getpid);
    uint64_t seen = 0;
    valid = read(link, &seen, sizeof(seen)) == (ssize_t)sizeof(seen) && seen > 0;
    close(link);
    close(prog);
    if (!valid) return -1;
    *program_id = info.id;
    *count = seen;
    return 0;
}
#endif
