/*
 * Firecracker guest PID1 WASIp1 verifier. Statically linked x86_64 Linux.
 *
 * Runs the REAL Wasmtime CLI inside guest rootfs and accepts execution
 * only when child exited zero AND returned the exact bytes produced by
 * wasi_snapshot_preview1.fd_write from the separately pinned WASM file.
 * PID1 remains alive for original dual-tenant KVM/cgroup/crash checks.
 * No networking, shell, mounts or access to the host broker.
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

static const char expected[]="OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE\n";
static void output(const char *s) { size_t n=strlen(s); while(n){ssize_t r=write(1,s,n); if(r<=0)break; s+=r;n-=(size_t)r;} }
static void fail(void) {output("OCAF039_GUEST_WASI_EXECUTION_FAILED\n");for(;;)pause();}

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
    output("OCAF039_GUEST_PID1_BOOTED\n");
    /* Nonblocking probe distinguishes an unseeded 4.14 guest kernel from
       normal Wasmtime startup. No RNG bytes are printed or persisted. */
    unsigned char entropy_probe[16];
    ssize_t gr=getrandom(entropy_probe,sizeof(entropy_probe),GRND_NONBLOCK);
    if(gr==(ssize_t)sizeof(entropy_probe)){
        output("OCAF039_GUEST_CRNG_READY\\n");
    }else{
        output("OCAF039_GUEST_CRNG_NOT_INITIALIZED\\n");
        if(seed_guest_crng()!=0){
            memset(entropy_probe,0,sizeof(entropy_probe));
            output("OCAF039_GUEST_HW_RNG_SEED_DENIED\\n");
            fail();
        }
        gr=getrandom(entropy_probe,sizeof(entropy_probe),GRND_NONBLOCK);
        if(gr!=(ssize_t)sizeof(entropy_probe)){
            memset(entropy_probe,0,sizeof(entropy_probe));
            output("OCAF039_GUEST_CRNG_STILL_UNAVAILABLE\\n");
            fail();
        }
        output("OCAF039_GUEST_CRNG_READY_AFTER_CPU_RNG\\n");
    }
    memset(entropy_probe,0,sizeof(entropy_probe));
    int channel[2]; if(pipe(channel)!=0)fail();
    pid_t child=fork();
    if(child<0)fail();
    if(child==0){
        close(channel[0]);
        if(dup2(channel[1],1)<0 || dup2(channel[1],2)<0)_exit(127);
        close(channel[1]);
        char * const args[]={"/usr/bin/wasmtime","run","-C","cache=no","/opt/ocaf/hello.wasm",NULL};
        char * const env[]={"PATH=/usr/bin:/bin","HOME=/","LANG=C",NULL};
        execve(args[0],args,env);
        _exit(127);
    }
    close(channel[1]);
    char response[8192];size_t count=0;
    while(count<sizeof(response)-1){
        ssize_t n=read(channel[0],response+count,sizeof(response)-1-count);
        if(n<0 && errno==EINTR)continue;
        if(n<=0)break;
        count+=(size_t)n;
    }
    close(channel[0]);
    response[count]=0;
    int status=0;pid_t waited;
    do{waited=waitpid(child,&status,0);}while(waited<0 && errno==EINTR);
    if(waited!=child||!WIFEXITED(status)||WEXITSTATUS(status)!=0||
       strcmp(response,expected)!=0) {
        output("OCAF039_GUEST_WASI_CHILD_ERROR_BEGIN\n");
        if(count)output(response);
        fail();
    }
    output(response);
    output("OCAF039_GUEST_WASI_EXECUTION_PASS_EXIT_0\n");
    for(;;)pause();
}
