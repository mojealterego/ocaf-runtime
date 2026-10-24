/* Firecracker guest negative WASIp1 budget runner. No receipt is printed
 * until real Wasmtime processes and a kernel seccomp filter have been tested.
 * The guest image builder embeds this static binary and both modules into the
 * signed read-only rootfs. PID1 requires the exact complete output and exit 0.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <poll.h>
#include <signal.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#ifndef OCAF_NEG_WASMTIME
#define OCAF_NEG_WASMTIME "/usr/bin/wasmtime"
#endif
#ifndef OCAF_NEG_SPIN
#define OCAF_NEG_SPIN "/opt/ocaf/negative-spin.wasm"
#endif
#ifndef OCAF_NEG_MEMORY
#define OCAF_NEG_MEMORY "/opt/ocaf/negative-memory.wasm"
#endif

static int denied_socket(void) {
    struct sock_filter rules[]={
        BPF_STMT(BPF_LD|BPF_W|BPF_ABS,offsetof(struct seccomp_data,nr)),
        BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K,SYS_socket,0,1),
        BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ERRNO|EPERM),
        BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ALLOW),
    };
    struct sock_fprog filter={.len=(unsigned short)(sizeof(rules)/sizeof(rules[0])),.filter=rules};
    if(prctl(PR_SET_NO_NEW_PRIVS,1,0,0,0)!=0 ||
       prctl(PR_SET_SECCOMP,SECCOMP_MODE_FILTER,&filter)!=0)return -1;
    errno=0;
    int fd=socket(AF_INET,SOCK_DGRAM,0);
    if(fd>=0){close(fd);return -1;}
    return errno==EPERM?0:-1;
}

static long elapsed_ms(struct timespec start) {
    struct timespec now;
    if(clock_gettime(CLOCK_MONOTONIC,&now)!=0)return -1;
    return (now.tv_sec-start.tv_sec)*1000L+(now.tv_nsec-start.tv_nsec)/1000000L;
}

static int run_case(char *const argv[],const char *trap,int cpu_signal,int parent_kill) {
    int channel[2];
    if(pipe(channel)!=0)return -1;
    pid_t child=fork();
    if(child<0){close(channel[0]);close(channel[1]);return -1;}
    if(child==0){
        close(channel[0]);
        if(setpgid(0,0)!=0 || dup2(channel[1],1)<0 || dup2(channel[1],2)<0)_exit(127);
        close(channel[1]);
        struct rlimit core={0,0},cpu={cpu_signal?1:5,cpu_signal?2:5};
        struct rlimit address={8ULL*1024*1024*1024,8ULL*1024*1024*1024};
        if(setrlimit(RLIMIT_CORE,&core)!=0 || setrlimit(RLIMIT_CPU,&cpu)!=0 ||
           setrlimit(RLIMIT_AS,&address)!=0 ||
           prctl(PR_SET_NO_NEW_PRIVS,1,0,0,0)!=0)_exit(127);
        char *const env[]={"PATH=/usr/bin:/bin","HOME=/","LANG=C",NULL};
        execve(argv[0],argv,env);
        _exit(127);
    }
    close(channel[1]);
    struct timespec start;
    if(clock_gettime(CLOCK_MONOTONIC,&start)!=0){kill(-child,SIGKILL);close(channel[0]);waitpid(child,NULL,0);return -1;}
    char output[8192];size_t used=0;int eof=0;
    for(;;){
        long elapsed=elapsed_ms(start);
        if(elapsed<0 || elapsed>=10000)break;
        if(parent_kill==1 && elapsed>=200){
            if(kill(child,0)!=0)break;
            if(kill(-child,SIGKILL)!=0)break;
            parent_kill=2;
        }
        struct pollfd item={.fd=channel[0],.events=POLLIN|POLLHUP};
        int wait_ms=parent_kill==1?200-(int)elapsed:100;
        if(wait_ms<1)wait_ms=1;
        int ready=poll(&item,1,wait_ms);
        if(ready<0 && errno==EINTR)continue;
        if(ready<0)break;
        if(ready==0)continue;
        ssize_t got=read(channel[0],output+used,sizeof(output)-1-used);
        if(got<0 && errno==EINTR)continue;
        if(got<0)break;
        if(got==0){eof=1;break;}
        used+=(size_t)got;
        if(used==sizeof(output)-1)break;
    }
    close(channel[0]);output[used]=0;
    if(!eof)kill(-child,SIGKILL);
    int status=0;pid_t waited;
    do{waited=waitpid(child,&status,0);}while(waited<0 && errno==EINTR);
    if(waited!=child || !eof)return -1;
    if(parent_kill)return parent_kill==2 && WIFSIGNALED(status) && WTERMSIG(status)==SIGKILL?0:-1;
    if(cpu_signal)return WIFSIGNALED(status) &&
        (WTERMSIG(status)==SIGXCPU || WTERMSIG(status)==SIGKILL)?0:-1;
    return ((WIFEXITED(status) && WEXITSTATUS(status)!=0) || WIFSIGNALED(status)) &&
        strstr(output,trap)!=NULL?0:-1;
}

int main(void) {
    if(denied_socket()!=0)return 2;
    puts("OCAF039_GUEST_SECCOMP_SOCKET_DENIED");fflush(stdout);
    char *fuel[]={OCAF_NEG_WASMTIME,"run","-C","cache=no","-W","fuel=10000",OCAF_NEG_SPIN,NULL};
    char *timeout[]={OCAF_NEG_WASMTIME,"run","-C","cache=no","-W","timeout=100ms",OCAF_NEG_SPIN,NULL};
    char *memory[]={OCAF_NEG_WASMTIME,"run","-C","cache=no","-W","max-memory-size=65536",
                    "-W","trap-on-grow-failure=y",OCAF_NEG_MEMORY,NULL};
    char *spin[]={OCAF_NEG_WASMTIME,"run","-C","cache=no",OCAF_NEG_SPIN,NULL};
    if(run_case(fuel,"wasm trap: all fuel consumed by WebAssembly",0,0)!=0)return 3;
    puts("OCAF039_GUEST_WASI_FUEL_TRAP");fflush(stdout);
    if(run_case(timeout,"wasm trap: interrupt",0,0)!=0)return 4;
    puts("OCAF039_GUEST_WASI_TIMEOUT_TRAP");fflush(stdout);
    if(run_case(memory,"forcing trap when growing memory to 196608 bytes",0,0)!=0)return 5;
    puts("OCAF039_GUEST_WASI_MEMORY_TRAP");fflush(stdout);
    if(run_case(spin,NULL,1,0)!=0)return 6;
    puts("OCAF039_GUEST_WASI_CPU_SIGNAL");fflush(stdout);
    if(run_case(spin,NULL,0,1)!=0)return 7;
    puts("OCAF039_GUEST_WASI_CHILD_SIGKILL");fflush(stdout);
    return 0;
}
