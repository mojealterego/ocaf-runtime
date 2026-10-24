/*
 * Firecracker guest PID1 WASIp1 verifier. Statically linked x86_64 Linux.
 *
 * Runs the REAL Wasmtime CLI inside guest rootfs and accepts execution
 * only when child exited zero AND returned the exact bytes produced by
 * wasi_snapshot_preview1.fd_write from the separately pinned WASM file.
 * PID1 remains alive for original dual-tenant KVM/cgroup/crash checks.
 * No networking, shell, or access to the host broker. The separate guest
 * probe helper mounts guest tracefs/debugfs when its kernel requires it.
 */
#define _GNU_SOURCE
#include <unistd.h>
#include <fcntl.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <errno.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <time.h>
#include <sys/random.h>
#include <sys/ioctl.h>
#include <linux/random.h>
#include <stdint.h>
#include <cpuid.h>
#include <linux/bpf.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <poll.h>

#ifndef OCAF_WASI_TENANT
#define OCAF_WASI_TENANT "legacy-test"
#endif
#ifndef OCAF_WASI_NONCE
#define OCAF_WASI_NONCE "00000000000000000000000000000000"
#endif
#ifndef OCAF_WASI_MODULE_SHA256
#define OCAF_WASI_MODULE_SHA256 "0000000000000000000000000000000000000000000000000000000000000000"
#endif

/* PID1 retains the attachment until its workload exits. This is a kernel
 * readiness proof, not a syscall probe or network egress policy. */
static int guest_bpf_program_fd=-1;
static int guest_bpf_socket_fd=-1;

/* Load and attach a real eBPF socket filter in the guest kernel. The host
 * TAP/TC classifier is a separate program and cannot satisfy this check. */
static int guest_bpf_socket_filter(void) {
    struct bpf_insn program[] = {
        {.code=BPF_ALU64|BPF_MOV|BPF_K, .dst_reg=BPF_REG_0, .imm=-1},
        {.code=BPF_JMP|BPF_EXIT},
    };
    const char license[]="GPL";
    union bpf_attr load={0};
    load.prog_type=BPF_PROG_TYPE_SOCKET_FILTER;
    load.insn_cnt=sizeof(program)/sizeof(program[0]);
    load.insns=(uint64_t)(uintptr_t)program;
    load.license=(uint64_t)(uintptr_t)license;
    int program_fd=(int)syscall(SYS_bpf,BPF_PROG_LOAD,&load,sizeof(load));
    if(program_fd<0)return -1;
    if(fcntl(program_fd,F_SETFD,FD_CLOEXEC)!=0){close(program_fd);return -1;}
    int sock=socket(AF_INET,SOCK_DGRAM|SOCK_CLOEXEC,0);
    if(sock<0 || setsockopt(sock,SOL_SOCKET,SO_ATTACH_BPF,&program_fd,sizeof(program_fd))!=0){
        if(sock>=0)close(sock);
        close(program_fd);
        return -1;
    }
    struct bpf_prog_info info={0};
    union bpf_attr query={0};
    query.info.bpf_fd=(uint32_t)program_fd;
    query.info.info_len=sizeof(info);
    query.info.info=(uint64_t)(uintptr_t)&info;
    int ok=(int)syscall(SYS_bpf,BPF_OBJ_GET_INFO_BY_FD,&query,sizeof(query));
    if(ok==0 && info.id>0){
        char marker[96];
        int length=snprintf(marker,sizeof(marker),"\nOCAF039_GUEST_BPF_SOCKET_FILTER_ID=%u\n",info.id);
        if(length<=0 || (size_t)length>=sizeof(marker) ||
           write(1,marker,(size_t)length)!=(ssize_t)length)ok=-1;
    }
    if(ok!=0 || info.id==0){close(sock);close(program_fd);return -1;}
    guest_bpf_socket_fd=sock;
    guest_bpf_program_fd=program_fd;
    return 0;
}

static void output(const char *s) { size_t n=strlen(s); while(n){ssize_t r=write(1,s,n); if(r<=0)break; s+=r;n-=(size_t)r;} }
static void fail(void) {output("OCAF039_GUEST_WASI_EXECUTION_FAILED\n");for(;;)pause();}

static int run_capture(char *const args[],char *response,size_t capacity) {
    int channel[2];if(pipe(channel)!=0)return -1;
    pid_t child=fork();
    if(child<0){close(channel[0]);close(channel[1]);return -1;}
    if(child==0){
        close(channel[0]);
        if(dup2(channel[1],1)<0 || dup2(channel[1],2)<0)_exit(127);
        close(channel[1]);
        char *const env[]={"PATH=/usr/bin:/bin","HOME=/","LANG=C",NULL};
        execve(args[0],args,env);
        _exit(127);
    }
    close(channel[1]);size_t count=0;
    while(count<capacity-1){
        ssize_t n=read(channel[0],response+count,capacity-1-count);
        if(n<0 && errno==EINTR)continue;
        if(n<=0)break;
        count+=(size_t)n;
    }
    close(channel[0]);response[count]=0;
    int status=0;pid_t waited;
    do{waited=waitpid(child,&status,0);}while(waited<0 && errno==EINTR);
    return waited==child && WIFEXITED(status) && WEXITSTATUS(status)==0 ? (int)count : -1;
}

static int read_probe_line(int fd,char *line,size_t capacity,int timeout_ms) {
    struct timespec start,now;
    if(clock_gettime(CLOCK_MONOTONIC,&start)!=0)return -1;
    size_t count=0;
    while(count+1<capacity){
        if(clock_gettime(CLOCK_MONOTONIC,&now)!=0)return -1;
        long elapsed=(now.tv_sec-start.tv_sec)*1000L+
                     (now.tv_nsec-start.tv_nsec)/1000000L;
        if(elapsed>=timeout_ms)return -1;
        struct pollfd waitfd={.fd=fd,.events=POLLIN|POLLHUP};
        int ready=poll(&waitfd,1,timeout_ms-(int)elapsed);
        if(ready<0 && errno==EINTR)continue;
        if(ready<=0)return -1;
        char c=0;
        ssize_t n=read(fd,&c,1);
        if(n<0 && errno==EINTR)continue;
        if(n!=1)return -1;
        if(c=='\n'){line[count]=0;return (int)count;}
        if(c<' ' || c>126)return -1;
        line[count++]=c;
    }
    return -1;
}

static int read_child_output(int fd,char *response,size_t capacity,int timeout_ms) {
    struct timespec start,now;
    if(clock_gettime(CLOCK_MONOTONIC,&start)!=0)return -1;
    size_t count=0;
    while(count+1<capacity){
        if(clock_gettime(CLOCK_MONOTONIC,&now)!=0)return -1;
        long elapsed=(now.tv_sec-start.tv_sec)*1000L+
                     (now.tv_nsec-start.tv_nsec)/1000000L;
        if(elapsed>=timeout_ms)return -1;
        struct pollfd waitfd={.fd=fd,.events=POLLIN|POLLHUP};
        int ready=poll(&waitfd,1,timeout_ms-(int)elapsed);
        if(ready<0 && errno==EINTR)continue;
        if(ready<=0)return -1;
        ssize_t n=read(fd,response+count,capacity-1-count);
        if(n<0 && errno==EINTR)continue;
        if(n<0)return -1;
        if(n==0){response[count]=0;return (int)count;}
        count+=(size_t)n;
    }
    return -1;
}

static void fail_children(pid_t wasi,pid_t probe) {
    if(wasi>0)kill(wasi,SIGKILL);
    if(probe>0)kill(probe,SIGKILL);
    if(wasi>0)waitpid(wasi,NULL,0);
    if(probe>0)waitpid(probe,NULL,0);
    fail();
}

/* Old upstream Linux 4.14 guests can block getrandom() indefinitely.
 * Never bypass CSPRNG with /dev/urandom; instead seed guest kernel with
 * actual RDSEED/RDRAND CPU entropy via privileged RNDADDENTROPY ioctl.
 * Fail closed when hardware randomness/capability is unavailable. */
static int seed_guest_crng(void) {
    unsigned int a,b,c,d;
    if(!__get_cpuid(1,&a,&b,&c,&d) || !(c&(1u<<30))) return -1;
    int rdseed=0;
    if(__get_cpuid_max(0,NULL)>=7 &&
       __get_cpuid_count(7,0,&a,&b,&c,&d)) rdseed=(b&(1u<<18))!=0;
    struct { int entropy_count; int buf_size; unsigned char buf[64]; } packet;
    memset(&packet,0,sizeof(packet));
    packet.entropy_count=256;packet.buf_size=sizeof(packet.buf);
    uint64_t last=0;
    for(int i=0;i<8;i++){
        uint64_t word=0;unsigned char ok=0;
        for(int n=0;n<64;n++){
            if(rdseed) __asm__ volatile("rdseed %0; setc %1"
                : "=r"(word), "=qm"(ok) :: "cc");
            else __asm__ volatile("rdrand %0; setc %1"
                : "=r"(word), "=qm"(ok) :: "cc");
            if(ok)break;
        }
        if(!ok || (i>0 && word==last)){
            memset(&packet,0,sizeof(packet));return -1;
        }
        memcpy(packet.buf+i*8,&word,8);
        last=word;
    }
    int fd=open("/dev/random",O_WRONLY|O_CLOEXEC);
    if(fd<0){memset(&packet,0,sizeof(packet));return -1;}
    int rv=ioctl(fd,RNDADDENTROPY,&packet);
    close(fd);
    memset(&packet,0,sizeof(packet));
    return rv==0?0:-1;
}

int main(void) {
    int fd=open("/dev/console",O_WRONLY|O_CLOEXEC);
    if(fd>=0){dup2(fd,1); dup2(fd,2);if(fd>2)close(fd);}
    output("\nOCAF039_GUEST_PID1_BOOTED\n");
    /* Nonblocking probe distinguishes an unseeded 4.14 guest kernel from
       normal Wasmtime startup. No RNG bytes are printed or persisted. */
    unsigned char entropy_probe[16];
    ssize_t gr=getrandom(entropy_probe,sizeof(entropy_probe),GRND_NONBLOCK);
    if(gr==(ssize_t)sizeof(entropy_probe)){
        output("\nOCAF039_GUEST_CRNG_READY\n");
    }else{
        output("OCAF039_GUEST_CRNG_NOT_INITIALIZED\n");
        if(seed_guest_crng()!=0){
            memset(entropy_probe,0,sizeof(entropy_probe));
            output("OCAF039_GUEST_HW_RNG_SEED_DENIED\n");
            fail();
        }
        gr=getrandom(entropy_probe,sizeof(entropy_probe),GRND_NONBLOCK);
        if(gr!=(ssize_t)sizeof(entropy_probe)){
            memset(entropy_probe,0,sizeof(entropy_probe));
            output("OCAF039_GUEST_CRNG_STILL_UNAVAILABLE\n");
            fail();
        }
        output("\nOCAF039_GUEST_CRNG_READY_AFTER_CPU_RNG\n");
    }
    memset(entropy_probe,0,sizeof(entropy_probe));
    if(guest_bpf_socket_filter()!=0){
        output("OCAF039_GUEST_BPF_LOAD_DENIED\n");
        fail();
    }
    char response[8192];
    char digest_expected[128];
    int digest_len=snprintf(digest_expected,sizeof(digest_expected),"%s  /opt/ocaf/hello.wasm\n",OCAF_WASI_MODULE_SHA256);
    char *const sha_args[]={"/usr/bin/sha256sum","/opt/ocaf/hello.wasm",NULL};
    int got=run_capture(sha_args,response,sizeof(response));
    if(digest_len<=0 || (size_t)digest_len>=sizeof(digest_expected) ||
       got!=digest_len || strcmp(response,digest_expected)!=0){
        output("OCAF039_GUEST_WASI_MODULE_DIGEST_DENIED\n");fail();
    }
    output("OCAF039_GUEST_WASI_MODULE_SHA256=");output(OCAF_WASI_MODULE_SHA256);output("\n");
    int start_pipe[2],wasi_pipe[2],probe_pipe[2];
    if(pipe(start_pipe)!=0 || pipe(wasi_pipe)!=0 || pipe(probe_pipe)!=0)fail();
    pid_t wasi=fork();
    if(wasi<0)fail();
    if(wasi==0){
        close(start_pipe[1]);close(wasi_pipe[0]);
        close(probe_pipe[0]);close(probe_pipe[1]);
        unsigned char release=0;
        ssize_t n;
        do{n=read(start_pipe[0],&release,1);}while(n<0 && errno==EINTR);
        close(start_pipe[0]);
        if(n!=1 || release!='G')_exit(127);
        if(dup2(wasi_pipe[1],1)<0 || dup2(wasi_pipe[1],2)<0)_exit(127);
        close(wasi_pipe[1]);
        char *const args[]={"/usr/bin/wasmtime","run","-C","cache=no","/opt/ocaf/hello.wasm",NULL};
        char *const env[]={"PATH=/usr/bin:/bin","HOME=/","LANG=C",NULL};
        execve(args[0],args,env);
        _exit(127);
    }
    close(start_pipe[0]);close(wasi_pipe[1]);
    char target[24];
    if(snprintf(target,sizeof(target),"%ld",(long)wasi)<=0)fail_children(wasi,-1);
    pid_t probe=fork();
    if(probe<0)fail_children(wasi,-1);
    if(probe==0){
        close(start_pipe[1]);close(wasi_pipe[0]);close(probe_pipe[0]);
        if(dup2(probe_pipe[1],1)<0 || dup2(probe_pipe[1],2)<0)_exit(127);
        close(probe_pipe[1]);
        char *const args[]={"/usr/bin/ocaf-guest-openat-probe",target,"30",NULL};
        char *const env[]={"PATH=/usr/bin:/bin","HOME=/","LANG=C",NULL};
        execve(args[0],args,env);
        _exit(127);
    }
    close(probe_pipe[1]);
    char line[256];unsigned int ready_id=0;int used=0;
    got=read_probe_line(probe_pipe[0],line,sizeof(line),30000);
    if(got<=0 || sscanf(line,"OCAF039_GUEST_OPENAT_PROBE_READY_ID=%u%n",&ready_id,&used)!=1 ||
       used!=got || ready_id==0){
        output("OCAF039_GUEST_OPENAT_PROBE_READY_DENIED\n");
        fail_children(wasi,probe);
    }
    output("\n");output(line);output("\n");
    if(write(start_pipe[1],"G",1)!=1)fail_children(wasi,probe);
    close(start_pipe[1]);
    got=read_child_output(wasi_pipe[0],response,sizeof(response),45000);
    close(wasi_pipe[0]);
    if(got<0)kill(wasi,SIGKILL);
    int wasi_status=0;pid_t waited;
    do{waited=waitpid(wasi,&wasi_status,0);}while(waited<0 && errno==EINTR);
    if(got<0 || waited!=wasi || !WIFEXITED(wasi_status) || WEXITSTATUS(wasi_status)!=0){
        output("OCAF039_GUEST_WASI_CHILD_ERROR_BEGIN\n");
        fail_children(-1,probe);
    }
    char expected[160];
    int expected_len=snprintf(expected,sizeof(expected),
        "OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE:%s:%s\n",OCAF_WASI_TENANT,OCAF_WASI_NONCE);
    if(got<0 || expected_len<=0 || (size_t)expected_len>=sizeof(expected) ||
       got!=expected_len || strcmp(response,expected)!=0) {
        output("OCAF039_GUEST_WASI_CHILD_ERROR_BEGIN\n");
        if(got>0)output(response);
        fail_children(-1,probe);
    }
    unsigned int event_id=0,tgid=0,pid=0;unsigned long long count=0;
    used=0;
    int line_length=read_probe_line(probe_pipe[0],line,sizeof(line),35000);
    close(probe_pipe[0]);
    if(line_length<=0 ||
       sscanf(line,"OCAF039_GUEST_OPENAT_EVENT_ID=%u COUNT=%llu TGID=%u PID=%u%n",
              &event_id,&count,&tgid,&pid,&used)!=4 ||
       used!=line_length || event_id!=ready_id || count==0 ||
       tgid!=(unsigned)wasi || pid==0){
        output("OCAF039_GUEST_OPENAT_EVENT_DENIED\n");
        fail_children(-1,probe);
    }
    int probe_status=0;
    do{waited=waitpid(probe,&probe_status,0);}while(waited<0 && errno==EINTR);
    if(waited!=probe || !WIFEXITED(probe_status) || WEXITSTATUS(probe_status)!=0){
        output("OCAF039_GUEST_OPENAT_PROBE_EXIT_DENIED\n");fail();
    }
    output("\n");output(line);output("\n");
    output("\n");
    output(response);
    output("OCAF039_GUEST_WASI_TENANT=");output(OCAF_WASI_TENANT);output("\n");
    output("OCAF039_GUEST_WASI_NONCE=");output(OCAF_WASI_NONCE);output("\n");
    output("\nOCAF039_GUEST_WASI_EXECUTION_PASS_EXIT_0\n");
    for(;;)pause();
}
