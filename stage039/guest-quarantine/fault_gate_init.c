/* Test-only PID1 gate: wait until the host Aya link is attached before
 * executing the real quarantine PID1 with its guest Aya probe removed. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/vm_sockets.h>
#include <stdio.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

int main(void) {
    int console=open("/dev/console",O_WRONLY|O_CLOEXEC);
    if(console>=0){dup2(console,1);dup2(console,2);if(console>2)close(console);}
    int fd=-1;
    struct sockaddr_vm host={.svm_family=AF_VSOCK,.svm_cid=VMADDR_CID_HOST,
                             .svm_port=5100};
    int connected=0;
    for(int i=0;i<100;i++){
        fd=socket(AF_VSOCK,SOCK_STREAM|SOCK_CLOEXEC,0);
        if(fd<0)return 100;
        if(connect(fd,(struct sockaddr *)&host,sizeof(host))==0){connected=1;break;}
        close(fd);
        struct timespec delay={.tv_sec=0,.tv_nsec=100000000};
        nanosleep(&delay,NULL);
    }
    if(!connected)return 101;
    if(write(fd,"R",1)!=1)return 102;
    char release=0;
    if(read(fd,&release,1)!=1 || release!='G')return 103;
    close(fd);
    puts("OCAF039_FAULT_HOST_AYA_GATE_RELEASED");fflush(stdout);
    char *const args[]={"/ocaf-q-run",NULL};
    execv(args[0],args);
    return 104;
}
