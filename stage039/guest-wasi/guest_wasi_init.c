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

static const char expected[]="OCAF039_WASI_INSIDE_FC_VM_IMPORT_FD_WRITE\n";
static void output(const char *s) { size_t n=strlen(s); while(n){ssize_t r=write(1,s,n); if(r<=0)break; s+=r;n-=(size_t)r;} }
static void fail(void) {output("OCAF039_GUEST_WASI_EXECUTION_FAILED\n");for(;;)pause();}

int main(void) {
    int fd=open("/dev/console",O_WRONLY|O_CLOEXEC);
    if(fd>=0){dup2(fd,1); dup2(fd,2);if(fd>2)close(fd);}
    output("OCAF039_GUEST_PID1_BOOTED\n");
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
