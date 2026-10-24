/*
 * Minimal Firecracker guest kernel tracepoint probe for sys_enter_openat.
 *
 * Build static: gcc -O2 -static -Wall -Wextra -Werror guest_openat_probe.c
 *   -o ocaf-guest-openat-probe
 *
 * PID1 must fork a blocked Wasmtime child, then start this helper with that
 * child's TGID and wait for READY before allowing execve. The helper accepts
 * only kernel events whose TGID matches the child and comm is "wasmtime".
 * It emits one EVENT with kernel BPF program ID, pid/tgid and count, then
 * exits. No host-side probe, guessed BPF ID, or synthetic event is accepted.
 *
 * This proves a guest kernel tracepoint event; it is not Aya, quarantine,
 * guest attestation, or an egress policy. Stage 0.39 remains open.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/bpf.h>
#include <linux/perf_event.h>
#include <linux/unistd.h>
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#define I(C,D,S,O,M) ((struct bpf_insn){.code=(C),.dst_reg=(D),.src_reg=(S),.off=(O),.imm=(M)})
#define MOV64I(D,V) I(BPF_ALU64|BPF_MOV|BPF_K,D,0,0,V)
#define MOV64R(D,S) I(BPF_ALU64|BPF_MOV|BPF_X,D,S,0,0)
#define STW(D,O,V) I(BPF_ST|BPF_W|BPF_MEM,D,0,O,V)
#define LDXW(D,S,O) I(BPF_LDX|BPF_W|BPF_MEM,D,S,O,0)
#define STXDW(D,S,O) I(BPF_STX|BPF_DW|BPF_MEM,D,S,O,0)
#define XADDDW(D,S,O) I(BPF_STX|BPF_DW|BPF_XADD,D,S,O,0)
#define JNEI(D,V,O) I(BPF_JMP|BPF_JNE|BPF_K,D,0,O,V)
#define JNER(D,S,O) I(BPF_JMP|BPF_JNE|BPF_X,D,S,O,0)
#define JEQ0(D,O) I(BPF_JMP|BPF_JEQ|BPF_K,D,0,O,0)
#define CALL(H) I(BPF_JMP|BPF_CALL,0,0,0,H)
#define EXIT I(BPF_JMP|BPF_EXIT,0,0,0,0)

struct counts {
    uint32_t target_tgid;
    uint32_t reserved;
    uint64_t count;
    uint64_t pid_tgid;
};

static int bpf_call(int command, union bpf_attr *attr) {
    return (int)syscall(SYS_bpf,command,attr,sizeof(*attr));
}

static int create_map(void) {
    union bpf_attr attr={0};
    attr.map_type=BPF_MAP_TYPE_ARRAY;
    attr.key_size=sizeof(uint32_t);
    attr.value_size=sizeof(struct counts);
    attr.max_entries=1;
    return bpf_call(BPF_MAP_CREATE,&attr);
}

/* eBPF verifier guarantees map-value pointer bounds. Helper 16 obtains the
 * current task comm, helper 1 resolves the array element, helper 14 obtains
 * pid/tgid. Offset targets below are documented in tests. */
static int load_program(int map_fd) {
    struct bpf_insn insns[]={
        MOV64R(BPF_REG_1,BPF_REG_10),                               /*  0 */
        I(BPF_ALU64|BPF_ADD|BPF_K,BPF_REG_1,0,0,-32),              /*  1 */
        MOV64I(BPF_REG_2,16),                                     /*  2 */
        CALL(BPF_FUNC_get_current_comm),                           /*  3 */
        JNEI(BPF_REG_0,0,24),                                     /*  4 -> exit */
        LDXW(BPF_REG_1,BPF_REG_10,-32),                            /*  5 */
        JNEI(BPF_REG_1,0x6d736177,22),                             /*  6: wasm */
        LDXW(BPF_REG_1,BPF_REG_10,-28),                            /*  7 */
        JNEI(BPF_REG_1,0x656d6974,20),                             /*  8: time */
        LDXW(BPF_REG_1,BPF_REG_10,-24),                            /*  9 */
        JNEI(BPF_REG_1,0,18),                                      /* 10: NUL */
        STW(BPF_REG_10,-4,0),                                     /* 11 */
        I(BPF_LD|BPF_DW|BPF_IMM,BPF_REG_1,BPF_PSEUDO_MAP_FD,0,map_fd), /* 12 */
        I(0,0,0,0,0),                                              /* 13 */
        MOV64R(BPF_REG_2,BPF_REG_10),                               /* 14 */
        I(BPF_ALU64|BPF_ADD|BPF_K,BPF_REG_2,0,0,-4),               /* 15 */
        CALL(BPF_FUNC_map_lookup_elem),                            /* 16 */
        JEQ0(BPF_REG_0,11),                                       /* 17 */
        MOV64R(BPF_REG_6,BPF_REG_0),                                /* 18 */
        CALL(BPF_FUNC_get_current_pid_tgid),                       /* 19 */
        MOV64R(BPF_REG_7,BPF_REG_0),                                /* 20 */
        MOV64R(BPF_REG_1,BPF_REG_0),                                /* 21 */
        I(BPF_ALU64|BPF_RSH|BPF_K,BPF_REG_1,0,0,32),              /* 22 */
        LDXW(BPF_REG_2,BPF_REG_6,0),                              /* 23 */
        JNER(BPF_REG_1,BPF_REG_2,4),                              /* 24 */
        MOV64I(BPF_REG_1,1),                                      /* 25 */
        XADDDW(BPF_REG_6,BPF_REG_1,8),                            /* 26 */
        STXDW(BPF_REG_6,BPF_REG_7,16),                            /* 27 */
        MOV64I(BPF_REG_0,0),                                      /* 28 */
        EXIT,                                                       /* 29 */
    };
    const char license[]="GPL";
    char log[65536]={0};
    union bpf_attr attr={0};
    attr.prog_type=BPF_PROG_TYPE_TRACEPOINT;
    attr.insn_cnt=(uint32_t)(sizeof(insns)/sizeof(insns[0]));
    attr.insns=(uint64_t)(uintptr_t)insns;
    attr.license=(uint64_t)(uintptr_t)license;
    attr.log_buf=(uint64_t)(uintptr_t)log;
    attr.log_size=sizeof(log);
    attr.log_level=1;
    int fd=bpf_call(BPF_PROG_LOAD,&attr);
    if(fd<0){
        int saved_errno=errno;
        fprintf(stderr,"OCAF039_GUEST_OPENAT_BPF_VERIFIER_DENIED_ERRNO=%d\n",saved_errno);
        fprintf(stderr,"OCAF039_GUEST_OPENAT_BPF_VERIFIER_LOG=%.2048s\n",log);
    }
    return fd;
}

static int read_tracepoint_id(void) {
    const char *paths[]={
        "/sys/kernel/tracing/events/syscalls/sys_enter_openat/id",
        "/sys/kernel/debug/tracing/events/syscalls/sys_enter_openat/id",
    };
    for(size_t i=0;i<sizeof(paths)/sizeof(paths[0]);i++){
        FILE *f=fopen(paths[i],"r");
        if(f){int id=0;int n=fscanf(f,"%d",&id);fclose(f);if(n==1 && id>0)return id;}
    }
    return -1;
}

static int make_tracepoint_id(void) {
    int id=read_tracepoint_id();if(id>0)return id;
    (void)mount("sysfs","/sys","sysfs",MS_NOSUID|MS_NODEV|MS_NOEXEC,NULL);
    id=read_tracepoint_id();if(id>0)return id;
    (void)mount("tracefs","/sys/kernel/tracing","tracefs",MS_NOSUID|MS_NODEV|MS_NOEXEC,NULL);
    id=read_tracepoint_id();if(id>0)return id;
    (void)mount("debugfs","/sys/kernel/debug","debugfs",MS_NOSUID|MS_NODEV|MS_NOEXEC,NULL);
    return read_tracepoint_id();
}

static int program_id(int fd) {
    struct bpf_prog_info info={0};
    union bpf_attr attr={0};
    attr.info.bpf_fd=(uint32_t)fd;
    attr.info.info_len=sizeof(info);
    attr.info.info=(uint64_t)(uintptr_t)&info;
    if(bpf_call(BPF_OBJ_GET_INFO_BY_FD,&attr)!=0 || info.id==0)return -1;
    return (int)info.id;
}

static int attach_tracepoint(int id,int prog_fd) {
    struct perf_event_attr attr={0};
    attr.type=PERF_TYPE_TRACEPOINT;
    attr.size=sizeof(attr);
    attr.config=(uint64_t)id;
    attr.sample_period=1;
    attr.wakeup_events=1;
    attr.disabled=1;
    int event_fd=(int)syscall(SYS_perf_event_open,&attr,-1,0,-1,PERF_FLAG_FD_CLOEXEC);
    if(event_fd<0)return -1;
    if(ioctl(event_fd,PERF_EVENT_IOC_SET_BPF,prog_fd)!=0 ||
       ioctl(event_fd,PERF_EVENT_IOC_ENABLE,0)!=0){close(event_fd);return -1;}
    return event_fd;
}

static int lookup_count(int map_fd,struct counts *result) {
    uint32_t key=0;
    union bpf_attr attr={0};
    attr.map_fd=(uint32_t)map_fd;
    attr.key=(uint64_t)(uintptr_t)&key;
    attr.value=(uint64_t)(uintptr_t)result;
    return bpf_call(BPF_MAP_LOOKUP_ELEM,&attr);
}

static int set_target(int map_fd,uint32_t tgid) {
    uint32_t key=0;
    struct counts value={.target_tgid=tgid};
    union bpf_attr attr={0};
    attr.map_fd=(uint32_t)map_fd;
    attr.key=(uint64_t)(uintptr_t)&key;
    attr.value=(uint64_t)(uintptr_t)&value;
    attr.flags=BPF_ANY;
    return bpf_call(BPF_MAP_UPDATE_ELEM,&attr);
}

static int timeout_seconds(const char *raw) {
    char *end=NULL;errno=0;long n=strtol(raw,&end,10);
    return errno==0 && end && *end==0 && n>=1 && n<=120 ? (int)n : -1;
}

int main(int argc,char **argv) {
    if(argc!=3){fputs("usage: ocaf-guest-openat-probe TARGET_TGID TIMEOUT_SECONDS\n",stderr);return 2;}
    /* Target PID is not a timeout: accept the positive Linux pid_t domain. */
    char *end=NULL;errno=0;long parsed=strtol(argv[1],&end,10);
    if(errno!=0 || !end || *end!=0 || parsed<=1 || parsed>2147483647L){
        fputs("invalid target TGID\n",stderr);return 2;
    }
    int tgid=(int)parsed;
    int seconds=timeout_seconds(argv[2]);
    if(seconds<0){fputs("invalid bounded timeout\n",stderr);return 2;}
    int trace_id=make_tracepoint_id();
    if(trace_id<=0){fputs("OCAF039_GUEST_OPENAT_TRACEPOINT_UNAVAILABLE\n",stderr);return 3;}
    /* Linux 4.14 charges BPF maps/programs to RLIMIT_MEMLOCK. A privileged
     * guest PID1 may raise it; a denied raise still leads to a real BPF load
     * attempt whose errno is reported below. */
    struct rlimit lock_limit={.rlim_cur=RLIM_INFINITY,.rlim_max=RLIM_INFINITY};
    (void)setrlimit(RLIMIT_MEMLOCK,&lock_limit);
    int map_fd=create_map();
    if(map_fd<0){perror("guest BPF map");return 3;}
    if(set_target(map_fd,(uint32_t)tgid)!=0){perror("guest BPF target map");close(map_fd);return 3;}
    int prog_fd=load_program(map_fd);
    if(prog_fd<0){close(map_fd);return 3;}
    int id=program_id(prog_fd);
    if(id<=0){fputs("OCAF039_GUEST_OPENAT_PROGRAM_ID_UNAVAILABLE\n",stderr);close(prog_fd);close(map_fd);return 3;}
    int event_fd=attach_tracepoint(trace_id,prog_fd);
    if(event_fd<0){perror("guest openat tracepoint attach");close(prog_fd);close(map_fd);return 3;}
    printf("OCAF039_GUEST_OPENAT_PROBE_READY_ID=%d\n",id);
    fflush(stdout);
    struct timespec start,now;
    clock_gettime(CLOCK_MONOTONIC,&start);
    do{
        struct counts counts={0};
        if(lookup_count(map_fd,&counts)!=0){perror("guest BPF map lookup");break;}
        if(counts.count>0 && (uint32_t)(counts.pid_tgid>>32)==(uint32_t)tgid){
            printf("OCAF039_GUEST_OPENAT_EVENT_ID=%d COUNT=%llu TGID=%u PID=%u\n",
                id,(unsigned long long)counts.count,
                (unsigned)(counts.pid_tgid>>32),(unsigned)counts.pid_tgid);
            fflush(stdout);
            close(event_fd);close(prog_fd);close(map_fd);
            return 0;
        }
        struct timespec nap={.tv_sec=0,.tv_nsec=20000000};
        nanosleep(&nap,NULL);
        clock_gettime(CLOCK_MONOTONIC,&now);
    }while(now.tv_sec-start.tv_sec<seconds);
    fputs("OCAF039_GUEST_OPENAT_WASMTIME_EVENT_TIMEOUT\n",stderr);
    close(event_fd);close(prog_fd);close(map_fd);
    return 4;
}
