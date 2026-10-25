/* One bounded, real guest-to-host filesystem boundary negative probe. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/mount.h>
#include <unistd.h>

#ifndef CANARY_NAME
#error CANARY_NAME required
#endif
#ifndef PROBE_NONCE
#error PROBE_NONCE required
#endif

static int forbidden_open(const char *path) {
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (fd >= 0) {
        close(fd);
        return 0;
    }
    return errno == ENOENT || errno == EACCES || errno == EPERM;
}

int main(void) {
    int console = open("/dev/console", O_WRONLY | O_NOCTTY);
    int proc = mount("proc", "/proc", "proc", MS_NOSUID | MS_NODEV | MS_NOEXEC, NULL);
    int proc_ready = proc == 0 || errno == EBUSY;
    char direct[96];
    char through_proc[128];
    int n1 = snprintf(direct, sizeof(direct), "/%s", CANARY_NAME);
    int n2 = snprintf(through_proc, sizeof(through_proc),
                      "/proc/1/root/%s", CANARY_NAME);
    int pass = console >= 0 && proc_ready && n1 > 0 && n1 < (int)sizeof(direct)
        && n2 > 0 && n2 < (int)sizeof(through_proc)
        && forbidden_open(direct) && forbidden_open(through_proc);
    if (console >= 0) {
        dprintf(console, "OCAF039_GUEST_HOST_CANARY_%s:%s\n",
                pass ? "DENIED" : "FAIL", PROBE_NONCE);
        close(console);
    }
    /* PID1 must remain live until the host validates the VMM and kills it. */
    for (;;) pause();
}
