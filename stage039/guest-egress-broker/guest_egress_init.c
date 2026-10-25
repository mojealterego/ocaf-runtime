/* Minimal PID1 for the isolated real-KVM vsock egress lab. */
#define _GNU_SOURCE
#include <sys/wait.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <signal.h>

static int invoke(void) {
    pid_t child=fork();
    if(child<0)return -1;
    if(child==0){
        char *const args[]={"/usr/bin/ocaf-guest-vsock-egress",NULL};
        char *const env[]={"PATH=/usr/bin:/bin","LANG=C",NULL};
        execve(args[0],args,env);
        _exit(127);
    }
    int status=0;
    if(waitpid(child,&status,0)!=child || !WIFEXITED(status))return -1;
    return WEXITSTATUS(status);
}

int main(void) {
    int fd=open("/dev/console",O_WRONLY);
    if(fd>=0){dup2(fd,1);dup2(fd,2);if(fd>2)close(fd);}
    puts("OCAF039_GUEST_VSOCK_PID1_BOOTED");fflush(stdout);
    int first=invoke();fflush(stdout);
    int second=invoke();fflush(stdout);
    int third=invoke();fflush(stdout);
    if(first==0 && second==4 && third==4){
        puts("OCAF039_GUEST_VSOCK_GRANT_CRASH_DENY_RESTART_REPLAY_DENY_PASS");
    }else{
        printf("OCAF039_GUEST_VSOCK_FAILURE first=%d second=%d third=%d\n",
               first,second,third);
    }
    fflush(stdout);
    for(;;)pause();
}
