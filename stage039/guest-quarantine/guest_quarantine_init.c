/* One-purpose Firecracker guest PID1: observe Wasmtime through Aya, then
 * accept a short-lived signed quarantine permit over AF_VSOCK. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/vm_sockets.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/random.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#define Q_PORT 5100u
#define MAX_FRAME 512u
static pid_t target_pid=-1;

static void reject(const char *reason) {
    printf("OCAF039_Q_GUEST_DENIED=%s\n",reason);fflush(stdout);
    if(target_pid>1){kill(target_pid,SIGKILL);waitpid(target_pid,NULL,0);}
    for(;;)pause();
}

static int line(int fd,char *out,size_t size,int seconds) {
    struct pollfd p={.fd=fd,.events=POLLIN};size_t n=0;
    while(n+1<size){
        int ready=poll(&p,1,seconds*1000);
        if(ready<0 && errno==EINTR)continue;
        if(ready<=0)return -1;
        char c;ssize_t r=read(fd,&c,1);
        if(r<0 && errno==EINTR)continue;
        if(r!=1)return -1;
        if(c=='\n'){out[n]=0;return (int)n;}
        if(c<' '||c>126)return -1;
        out[n++]=c;
    }
    return -1;
}

static int exact(int fd,void *buffer,size_t size,int write_mode,int timeout_ms) {
    unsigned char *p=buffer;
    while(size){
        struct pollfd f={.fd=fd,.events=write_mode?POLLOUT:POLLIN};
        int ready=poll(&f,1,timeout_ms);
        if(ready<0 && errno==EINTR)continue;
        if(ready<=0)return -1;
        ssize_t n=write_mode?send(fd,p,size,MSG_NOSIGNAL):recv(fd,p,size,0);
        if(n<0 && errno==EINTR)continue;
        if(n<=0)return -1;
        p+=(size_t)n;size-=(size_t)n;
    }
    return 0;
}

static int process_ticks(pid_t pid,unsigned long long *ticks){
    char path[64],raw[2048];
    if(snprintf(path,sizeof(path),"/proc/%ld/stat",(long)pid)<=0)return -1;
    int fd=open(path,O_RDONLY|O_CLOEXEC|O_NOFOLLOW);
    if(fd<0)return -1;
    ssize_t n=read(fd,raw,sizeof(raw)-1);close(fd);
    if(n<=0 || (size_t)n>=sizeof(raw)-1)return -1;
    raw[n]=0;char *end=strrchr(raw,')');
    if(!end || end[1]!=' ')return -1;
    char *save=NULL,*item=strtok_r(end+2," ",&save);
    for(int field=3;field<=22;field++){
        if(!item)return -1;
        if(field==22){
            char *stop=NULL;errno=0;*ticks=strtoull(item,&stop,10);
            return errno || !stop || *stop || !*ticks ? -1:0;
        }
        item=strtok_r(NULL," ",&save);
    }
    return -1;
}

static int store(const char *name,const unsigned char *data,size_t size){
    int fd=open(name,O_CREAT|O_EXCL|O_WRONLY|O_CLOEXEC,0400);
    if(fd<0)return -1;
    size_t offset=0;
    while(offset<size){ssize_t n=write(fd,data+offset,size-offset);
        if(n<0 && errno==EINTR)continue;
        if(n<=0){close(fd);return -1;}offset+=(size_t)n;}
    int result=fsync(fd);close(fd);return result;
}

int main(void){
    int console=open("/dev/console",O_WRONLY|O_CLOEXEC);
    if(console>=0){dup2(console,1);dup2(console,2);if(console>2)close(console);}
    puts("\nOCAF039_Q_GUEST_PID1_BOOTED");fflush(stdout);
    if(mount("proc","/proc","proc",MS_NOSUID|MS_NODEV|MS_NOEXEC,NULL)!=0 && errno!=EBUSY)
        reject("proc_mount");
    if(mount("tmpfs","/run","tmpfs",MS_NOSUID|MS_NODEV,"size=64k,mode=0700")!=0)
        reject("run_mount");
    int gate[2],probe_pipe[2];
    if(pipe(gate)!=0 || pipe(probe_pipe)!=0)reject("pipe");
    pid_t target=fork();
    if(target<0)reject("fork_target");
    if(target==0){
        close(gate[1]);close(probe_pipe[0]);close(probe_pipe[1]);
        char release;ssize_t n=read(gate[0],&release,1);
        if(n!=1 || release!='G')_exit(127);
        close(gate[0]);
        char *const args[]={"/usr/bin/wasmtime","run","-C","cache=no",
                            "/opt/ocaf/quarantine-spin.wasm",NULL};
        char *const env[]={"PATH=/usr/bin:/bin","HOME=/","LANG=C",NULL};
        execve(args[0],args,env);_exit(127);
    }
    target_pid=target;
    close(gate[0]);
    char pid_text[32];snprintf(pid_text,sizeof(pid_text),"%ld",(long)target);
    pid_t probe=fork();
    if(probe<0){kill(target,SIGKILL);reject("fork_probe");}
    if(probe==0){
        close(gate[1]);dup2(probe_pipe[1],1);close(probe_pipe[0]);close(probe_pipe[1]);
        char *const args[]={"/usr/bin/ocaf-guest-openat-probe",pid_text,"30","2",NULL};
        execv(args[0],args);_exit(127);
    }
    close(probe_pipe[1]);
    char reply[256];unsigned ready_id=0,event_id=0,tgid=0,event_pid=0;
    unsigned long long count=0,ticks=0;
    int used=0,n=line(probe_pipe[0],reply,sizeof(reply),30);
    if(n<=0 || sscanf(reply,"OCAF039_GUEST_OPENAT_PROBE_READY_ID=%u%n",&ready_id,&used)!=1
       || used!=n || !ready_id)reject("aya_ready");
    if(write(gate[1],"G",1)!=1)reject("release_target");
    close(gate[1]);
    n=line(probe_pipe[0],reply,sizeof(reply),30);used=0;
    if(n<=0 || sscanf(reply,"OCAF039_GUEST_OPENAT_EVENT_ID=%u COUNT=%llu TGID=%u PID=%u%n",
       &event_id,&count,&tgid,&event_pid,&used)!=4 || used!=n || event_id!=ready_id
       || count<2 || tgid!=(unsigned)target || !event_pid)reject("aya_event");
    close(probe_pipe[0]);int status=0;
    if(waitpid(probe,&status,0)!=probe || !WIFEXITED(status)||WEXITSTATUS(status)!=0)
        reject("aya_exit");
    if(kill(target,0)!=0 || process_ticks(target,&ticks)!=0)reject("target_identity");
    char boot[64],nonce[33];int boot_fd=open("/proc/sys/kernel/random/boot_id",O_RDONLY|O_CLOEXEC);
    if(boot_fd<0)reject("boot_identity");
    ssize_t boot_len=read(boot_fd,boot,sizeof(boot)-1);close(boot_fd);
    if(boot_len<=0 || boot_len>=(ssize_t)sizeof(boot))reject("boot_identity");
    boot[boot_len]=0;boot[strcspn(boot,"\n")]=0;
    unsigned char random[16];
    ssize_t random_len=getrandom(random,sizeof(random),GRND_NONBLOCK);
    if(random_len!=(ssize_t)sizeof(random))reject("nonce_rng");
    for(size_t i=0;i<sizeof(random);i++)snprintf(nonce+i*2,3,"%02x",random[i]);
    char challenge[256];n=snprintf(challenge,sizeof(challenge),
        "OCAF039QC1|tenant-a|%s|%s|%ld|%llu|%u|%llu\n",
        nonce,boot,(long)target,ticks,ready_id,count);
    if(n<=0 || (size_t)n>=sizeof(challenge))reject("challenge_bound");
    printf("OCAF039_Q_AYA_ID=%u COUNT=%llu PID=%ld\n",ready_id,count,(long)target);
    fflush(stdout);
    int sock=socket(AF_VSOCK,SOCK_STREAM|SOCK_CLOEXEC,0);
    if(sock<0)reject("vsock_create");
    struct sockaddr_vm peer={.svm_family=AF_VSOCK,.svm_cid=VMADDR_CID_HOST,.svm_port=Q_PORT};
    if(connect(sock,(struct sockaddr *)&peer,sizeof(peer))!=0)reject("vsock_connect");
    unsigned char prefix[2]={(unsigned char)(n>>8),(unsigned char)n};
    if(exact(sock,prefix,2,1,5000)!=0 || exact(sock,challenge,(size_t)n,1,5000)!=0)
        reject("challenge_send");
    if(exact(sock,prefix,2,0,45000)!=0)reject("permit_receive");
    size_t permit_len=((size_t)prefix[0]<<8)|prefix[1];
    unsigned char permit[MAX_FRAME],signature[64];
    if(!permit_len || permit_len>=MAX_FRAME || exact(sock,permit,permit_len,0,5000)!=0 ||
       exact(sock,signature,sizeof(signature),0,5000)!=0)reject("permit_receive");
    close(sock);
    if(store("/run/permit",permit,permit_len)!=0 ||
       store("/run/signature",signature,sizeof(signature))!=0)reject("permit_store");
    pid_t helper=fork();
    if(helper<0)reject("fork_helper");
    if(helper==0){char *const args[]={"/usr/bin/ocaf-guest-quarantine",
        "/run/permit","/run/signature",NULL};execv(args[0],args);_exit(127);}
    if(waitpid(helper,&status,0)!=helper || !WIFEXITED(status)||WEXITSTATUS(status)!=0)
        reject("quarantine_helper");
    if(kill(target,0)!=0)reject("target_died");
    puts("OCAF039_Q_GUEST_AUTHORIZED_STOP_RELEASE_PASS");fflush(stdout);
    kill(target,SIGKILL);waitpid(target,NULL,0);
    for(;;)pause();
}
