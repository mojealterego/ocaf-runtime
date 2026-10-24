/* Actual Linux seccomp denial, suitable for inclusion in a guest image.
 * This is a separate negative probe, not proof of Firecracker integration. */
#define _GNU_SOURCE
#include <errno.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <stddef.h>
#include <stdio.h>
#include <sys/prctl.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <unistd.h>

int main(void) {
    struct sock_filter rules[] = {
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_socket, 0, 1),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | EPERM),
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
    };
    struct sock_fprog filter = {
        .len = (unsigned short)(sizeof(rules) / sizeof(rules[0])),
        .filter = rules,
    };
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 ||
        prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &filter) != 0) {
        fputs("OCAF039_SECCOMP_INSTALL_DENIED\n", stderr);
        return 2;
    }
    errno = 0;
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd >= 0) {
        close(fd);
        fputs("OCAF039_SECCOMP_SOCKET_ALLOWED_UNEXPECTEDLY\n", stderr);
        return 3;
    }
    if (errno != EPERM) {
        fputs("OCAF039_SECCOMP_SOCKET_WRONG_ERRNO\n", stderr);
        return 4;
    }
    puts("OCAF039_SECCOMP_SOCKET_DENIED");
    return 0;
}
