/*
 * Guest-side egress client. Only AF_VSOCK CID 2 / port 5000 is reachable;
 * no AF_INET socket, DNS, redirect, or HTTPS client exists in this process.
 * The host bridge authenticates the Firecracker process and verifies the
 * signed destination permit before making any network request.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>
#include <linux/vm_sockets.h>

#define MAX_PERMIT (64u * 1024u)
#define MAX_REPLY (1024u * 1024u)
#define HOST_PORT 5000u
#define PERMIT_FILE "/opt/ocaf/egress-permit.json"
#define PARTITION_PERMIT "/opt/ocaf/egress-permit-partition.json"
#define RECOVERY_PERMIT "/opt/ocaf/egress-permit-recovery.json"

static void deny(void) { fputs("OCAF039_GUEST_EGRESS_DENIED\n", stdout); }

static int wait_fd(int fd,short events,int timeout_ms) {
    struct pollfd p={.fd=fd,.events=events};
    int n;
    do { n=poll(&p,1,timeout_ms); } while(n<0 && errno==EINTR);
    return n==1 && (p.revents & events) ? 0 : -1;
}

static int64_t now_ms(void) {
    struct timespec now;
    if(clock_gettime(CLOCK_MONOTONIC,&now)!=0)return -1;
    return (int64_t)now.tv_sec*1000+now.tv_nsec/1000000;
}

static int exact_io(int fd,void *buffer,size_t size,int sending,int64_t deadline_ms) {
    unsigned char *p=buffer;
    while(size>0){
        int64_t remaining=deadline_ms-now_ms();
        if(remaining<=0 || remaining>12000 ||
           wait_fd(fd,sending?POLLOUT:POLLIN,(int)remaining)!=0)return -1;
        ssize_t n=sending?send(fd,p,size,MSG_NOSIGNAL):recv(fd,p,size,0);
        if(n<0 && errno==EINTR)continue;
        if(n<=0)return -1;
        p+=(size_t)n;size-=(size_t)n;
    }
    return 0;
}

static int read_permit(const char *path,unsigned char *data,size_t *length) {
    int fd=open(path,O_RDONLY|O_CLOEXEC|O_NOFOLLOW);
    if(fd<0)return -1;
    struct stat st;
    if(fstat(fd,&st)!=0 || !S_ISREG(st.st_mode) || st.st_nlink!=1 ||
       st.st_size<=0 || st.st_size>MAX_PERMIT || (st.st_mode & 0022)){
        close(fd);return -1;
    }
    size_t needed=(size_t)st.st_size,count=0;
    if(needed>MAX_PERMIT){close(fd);return -1;}
    while(count<needed){
        unsigned char block[4096];
        size_t request=needed-count;
        if(request>sizeof(block))request=sizeof(block);
        ssize_t n=read(fd,block,request);
        if(n<0 && errno==EINTR)continue;
        if(n<=0){close(fd);return -1;}
        if((size_t)n>MAX_PERMIT-count){close(fd);return -1;}
        memcpy(data+count,block,(size_t)n);
        count+=(size_t)n;
    }
    unsigned char extra;
    ssize_t n=read(fd,&extra,1);
    close(fd);
    if(n!=0)return -1;
    *length=needed;
    return 0;
}

int main(int argc,char **argv) {
    const char *permit_path=PERMIT_FILE;
    if(argc==2 && (!strcmp(argv[1],PARTITION_PERMIT) ||
                   !strcmp(argv[1],RECOVERY_PERMIT)))permit_path=argv[1];
    else if(argc!=1){deny();return 4;}
    unsigned char permit[MAX_PERMIT];size_t permit_len=0;
    if(read_permit(permit_path,permit,&permit_len)!=0){deny();return 4;}
    int fd=socket(AF_VSOCK,SOCK_STREAM|SOCK_CLOEXEC|SOCK_NONBLOCK,0);
    if(fd<0){deny();return 4;}
    struct sockaddr_vm address={.svm_family=AF_VSOCK,
                                .svm_cid=VMADDR_CID_HOST,.svm_port=HOST_PORT};
    int rc=connect(fd,(struct sockaddr *)&address,sizeof(address));
    if(rc!=0 && errno!=EINPROGRESS){close(fd);deny();return 4;}
    if(rc!=0){
        int err=0;socklen_t size=sizeof(err);
        if(wait_fd(fd,POLLOUT,3000)!=0 ||
           getsockopt(fd,SOL_SOCKET,SO_ERROR,&err,&size)!=0 || err!=0){
            close(fd);deny();return 4;
        }
    }
    unsigned char header[4]={
        (unsigned char)(permit_len>>24),(unsigned char)(permit_len>>16),
        (unsigned char)(permit_len>>8),(unsigned char)permit_len};
    int64_t started=now_ms();
    if(started<0){close(fd);deny();return 4;}
    int64_t send_deadline=started+3000;
    if(exact_io(fd,header,sizeof(header),1,send_deadline)!=0 ||
       exact_io(fd,permit,permit_len,1,send_deadline)!=0){close(fd);deny();return 4;}
    unsigned char response[5];
    started=now_ms();
    if(started<0){close(fd);deny();return 4;}
    int64_t reply_deadline=started+12000;
    if(exact_io(fd,response,sizeof(response),0,reply_deadline)!=0){close(fd);deny();return 4;}
    uint32_t length=((uint32_t)response[1]<<24)|((uint32_t)response[2]<<16)|
                    ((uint32_t)response[3]<<8)|response[4];
    if(response[0]!=1 || length>MAX_REPLY){close(fd);deny();return 4;}
    unsigned char chunk[4096];uint32_t remaining=length;
    while(remaining>0){
        size_t take=remaining<sizeof(chunk)?remaining:sizeof(chunk);
        if(exact_io(fd,chunk,take,0,reply_deadline)!=0){close(fd);deny();return 4;}
        remaining-=(uint32_t)take;
    }
    close(fd);
    printf("OCAF039_GUEST_EGRESS_BROKERED_BYTES=%u\n",length);
    return 0;
}
