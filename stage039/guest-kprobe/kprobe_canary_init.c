/* A bounded guest PID1 proof: host-only canary denied, BTF present, and a
 * kprobe BPF program actually attached and triggered in the guest kernel. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/bpf.h>
#include <linux/perf_event.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef CANARY_NAME
#error CANARY_NAME required
#endif
#ifndef PROBE_NONCE
#error PROBE_NONCE required
#endif

static const char *phase = "startup";
static void stop_forever(void) { for (;;) pause(); }
static int read_u32(const char *path, unsigned int *number) {
    char data[32] = {0};
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) return -1;
    ssize_t count = read(fd, data, sizeof(data) - 1);
    close(fd);
    if (count <= 0 || count >= (ssize_t)sizeof(data)) return -1;
    return sscanf(data, "%u", number) == 1 ? 0 : -1;
}
static int denied(const char *path, const char *stage) {
    phase = stage;
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (fd >= 0) { close(fd); return 0; }
    return errno == ENOENT || errno == EACCES || errno == EPERM;
}
static int proof(unsigned int *program_id, uint64_t *events) {
    phase = "mount_proc";
    if (mount("proc", "/proc", "proc", MS_NOSUID | MS_NODEV | MS_NOEXEC, NULL) != 0
            && errno != EBUSY) return -1;
    phase = "mount_sysfs";
    if (mount("sysfs", "/sys", "sysfs", MS_NOSUID | MS_NODEV | MS_NOEXEC, NULL) != 0
            && errno != EBUSY) return -1;
    unsigned char magic[2];
    phase = "open_btf";
    int btf = open("/sys/kernel/btf/vmlinux", O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (btf < 0) return -1;
    phase = "read_btf_magic";
    int btf_ok = read(btf, magic, sizeof(magic)) == (ssize_t)sizeof(magic)
              && magic[0] == 0x9f && magic[1] == 0xeb;
    close(btf);
    if (!btf_ok) return -1;
    phase = "mount_tracefs";
    if (mount("tracefs", "/sys/kernel/tracing", "tracefs",
              MS_NOSUID | MS_NODEV | MS_NOEXEC, NULL) != 0 && errno != EBUSY) return -1;
    phase = "open_kprobe_events";
    int control = open("/sys/kernel/tracing/kprobe_events", O_WRONLY | O_CLOEXEC | O_NOFOLLOW);
    if (control < 0) return -1;
    const char *event = "p:ocaf039_getpid __x64_sys_getpid\n";
    phase = "create_kprobe_event";
    int created = write(control, event, strlen(event)) == (ssize_t)strlen(event);
    close(control);
    if (!created) return -1;
    unsigned int event_id = 0;
    phase = "read_kprobe_event_id";
    if (read_u32("/sys/kernel/tracing/events/kprobes/ocaf039_getpid/id", &event_id) != 0
            || event_id == 0) return -1;

    struct bpf_insn instructions[] = {
        /* A tracepoint-attached kprobe program returns 1 to retain events;
         * returning 0 filters them and leaves perf's count at zero. */
        {.code = BPF_ALU64 | BPF_MOV | BPF_K, .dst_reg = BPF_REG_0, .imm = 1},
        {.code = BPF_JMP | BPF_EXIT},
    };
    static const char license[] = "GPL";
    union bpf_attr attr = {0};
    attr.prog_type = BPF_PROG_TYPE_KPROBE;
    attr.insn_cnt = sizeof(instructions) / sizeof(instructions[0]);
    attr.insns = (uint64_t)(uintptr_t)instructions;
    attr.license = (uint64_t)(uintptr_t)license;
    phase = "bpf_prog_load";
    int program = syscall(SYS_bpf, BPF_PROG_LOAD, &attr, sizeof(attr));
    if (program < 0) return -1;
    struct bpf_prog_info info = {0};
    memset(&attr, 0, sizeof(attr));
    attr.info.bpf_fd = program;
    attr.info.info_len = sizeof(info);
    attr.info.info = (uint64_t)(uintptr_t)&info;
    phase = "bpf_prog_info";
    if (syscall(SYS_bpf, BPF_OBJ_GET_INFO_BY_FD, &attr, sizeof(attr)) < 0 || info.id == 0) {
        close(program); return -1;
    }
    struct perf_event_attr perf = {0};
    perf.type = PERF_TYPE_TRACEPOINT;
    perf.size = sizeof(perf);
    perf.config = event_id;
    perf.sample_period = 1;
    perf.disabled = 1;
    phase = "perf_event_open";
    int link = syscall(SYS_perf_event_open, &perf, -1, 0, -1, PERF_FLAG_FD_CLOEXEC);
    if (link < 0) { close(program); return -1; }
    phase = "perf_attach_bpf";
    if (ioctl(link, PERF_EVENT_IOC_SET_BPF, program) != 0) {
        close(link); close(program); return -1;
    }
    phase = "perf_enable";
    if (ioctl(link, PERF_EVENT_IOC_ENABLE, 0) != 0) {
        close(link); close(program); return -1;
    }
    phase = "trigger_getpid";
    for (int i = 0; i < 64; i++) (void)syscall(SYS_getpid);
    uint64_t count = 0;
    phase = "read_perf_count";
    int observed = read(link, &count, sizeof(count)) == (ssize_t)sizeof(count) && count > 0;
    close(link);
    close(program);
    if (!observed) return -1;
    *program_id = info.id;
    *events = count;
    return 0;
}

int main(void) {
    int console = open("/dev/console", O_WRONLY | O_NOCTTY);
    char direct[96], through_proc[128];
    int n1 = snprintf(direct, sizeof(direct), "/%s", CANARY_NAME);
    int n2 = snprintf(through_proc, sizeof(through_proc), "/proc/1/root/%s", CANARY_NAME);
    unsigned int program_id = 0;
    uint64_t events = 0;
    int ok = console >= 0 && n1 > 0 && n1 < (int)sizeof(direct)
          && n2 > 0 && n2 < (int)sizeof(through_proc)
          && proof(&program_id, &events) == 0
          && denied(direct, "canary_direct")
          && denied(through_proc, "canary_proc_root");
    if (console >= 0) {
        if (ok) {
            dprintf(console, "OCAF039_GUEST_KPROBE_PASS program_id=%u event_count=%llu\n",
                    program_id, (unsigned long long)events);
            dprintf(console, "OCAF039_GUEST_HOST_CANARY_DENIED:%s\n", PROBE_NONCE);
        } else {
            dprintf(console, "OCAF039_GUEST_KPROBE_FAIL:%s phase=%s errno=%d\n",
                    PROBE_NONCE, phase, errno);
        }
        close(console);
    }
    stop_forever();
}
