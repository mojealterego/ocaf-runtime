/* Reversible guest PID quarantine, gated by an Ed25519 operator permit.
 *
 * This helper is intentionally independent of the WASI child and must be
 * invoked only after a trusted controller has corroborated the guest syscall
 * event with its host evidence. The permit's authority key is pinned inside
 * the read-only guest image. No heuristic filename or entropy signal is an
 * authorization to stop a process.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <openssl/evp.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#ifndef OCAF_Q_TENANT
#define OCAF_Q_TENANT "tenant-a"
#endif
#ifndef OCAF_Q_TRUSTED_KEY
#define OCAF_Q_TRUSTED_KEY "/opt/ocaf/quarantine-authority.pub"
#endif

#define MAX_PERMIT 512

static int read_exact_file(const char *path,unsigned char *data,size_t want,size_t *got) {
    int fd=open(path,O_RDONLY|O_CLOEXEC|O_NOFOLLOW);
    if(fd<0)return -1;
    struct stat st;
    if(fstat(fd,&st)!=0 || !S_ISREG(st.st_mode) || st.st_nlink!=1 ||
       (st.st_mode&022)!=0 || st.st_size<=0 || (size_t)st.st_size>want){close(fd);return -1;}
    size_t count=0;
    while(count<(size_t)st.st_size){
        ssize_t n=read(fd,data+count,(size_t)st.st_size-count);
        if(n<0 && errno==EINTR)continue;
        if(n<=0){close(fd);return -1;}
        count+=(size_t)n;
    }
    close(fd);*got=count;return 0;
}

static int signed_permit(const char *permit_path,const char *sig_path,char *buffer,size_t size) {
    unsigned char public_key[32],signature[64];
    size_t key_len=0,sig_len=0,payload_len=0;
    if(read_exact_file(OCAF_Q_TRUSTED_KEY,public_key,sizeof(public_key),&key_len)!=0 || key_len!=32 ||
       read_exact_file(sig_path,signature,sizeof(signature),&sig_len)!=0 || sig_len!=64 ||
       read_exact_file(permit_path,(unsigned char *)buffer,size-1,&payload_len)!=0 ||
       payload_len==0 || payload_len>=size)return -1;
    EVP_PKEY *key=EVP_PKEY_new_raw_public_key(EVP_PKEY_ED25519,NULL,public_key,sizeof(public_key));
    if(!key)return -1;
    EVP_MD_CTX *ctx=EVP_MD_CTX_new();
    int okay=ctx && EVP_DigestVerifyInit(ctx,NULL,NULL,NULL,key)==1 &&
             EVP_DigestVerify(ctx,signature,sig_len,(unsigned char *)buffer,payload_len)==1;
    EVP_MD_CTX_free(ctx);EVP_PKEY_free(key);
    if(!okay)return -1;
    buffer[payload_len]=0;
    return (int)payload_len;
}

static int hex_string(const char *text,size_t length) {
    if(strlen(text)!=length)return 0;
    for(size_t i=0;i<length;i++)
        if(!((text[i]>='0'&&text[i]<='9')||(text[i]>='a'&&text[i]<='f')))return 0;
    return 1;
}

static int number(const char *raw,uint64_t *out) {
    if(!raw || !*raw || (*raw=='0'&&raw[1]) || *raw=='+' || *raw=='-')return -1;
    for(const char *p=raw;*p;p++)if(*p<'0'||*p>'9')return -1;
    errno=0;char *end=NULL;unsigned long long value=strtoull(raw,&end,10);
    if(errno || !end || *end)return -1;
    *out=(uint64_t)value;return 0;
}

struct permit {
    pid_t pid;
    uint64_t start_ticks;
    uint64_t hold_ms;
};

static int parse_permit(char *data,size_t len,struct permit *out) {
    if(len==0 || data[len-1]!='\n')return -1;
    data[len-1]=0;
    char *fields[12];size_t count=0;char *cursor=data;
    while(count<12){
        fields[count++]=cursor;
        char *pipe=strchr(cursor,'|');
        if(!pipe)break;
        *pipe=0;cursor=pipe+1;
    }
    if(count!=12 || strchr(fields[11],'|') || strcmp(fields[0],"OCAF039Q1") ||
       strcmp(fields[1],OCAF_Q_TENANT) || !hex_string(fields[2],32) ||
       !hex_string(fields[11],64))return -1;
    /* Exact guest kernel identity defeats a signed permit replay on reboot. */
    char boot_id[64];FILE *f=fopen("/proc/sys/kernel/random/boot_id","r");
    if(!f)return -1;
    char *read=fgets(boot_id,sizeof(boot_id),f);fclose(f);
    if(!read)return -1;
    boot_id[strcspn(boot_id,"\n")]=0;
    if(strcmp(fields[3],boot_id))return -1;
    uint64_t values[7];
    for(size_t i=0;i<7;i++)if(number(fields[4+i],&values[i])!=0)return -1;
    /* pid, start ticks, guest BPF ID, host BPF ID, count, hold ms, expiry. */
    if(values[0]<=1 || values[0]>INT32_MAX || values[1]==0 ||
       values[2]==0 || values[2]>UINT32_MAX || values[3]==0 || values[3]>UINT32_MAX ||
       values[4]<2 || values[5]<10 || values[5]>5000)return -1;
    time_t now=time(NULL);
    if(now<0 || values[6]<=(uint64_t)now || values[6]>(uint64_t)now+300)return -1;
    int any=0;for(const char *p=fields[11];*p;p++)if(*p!='0'){any=1;break;}
    if(!any)return -1;
    out->pid=(pid_t)values[0];out->start_ticks=values[1];out->hold_ms=values[5];
    return 0;
}

/* Field 22 of /proc/<pid>/stat. Use the final ')' to handle spaces in comm. */
static int start_ticks(pid_t pid,uint64_t *result) {
    char path[64],data[2048];
    int n=snprintf(path,sizeof(path),"/proc/%ld/stat",(long)pid);
    if(n<=0 || (size_t)n>=sizeof(path))return -1;
    int fd=open(path,O_RDONLY|O_CLOEXEC|O_NOFOLLOW);
    if(fd<0)return -1;
    ssize_t size=read(fd,data,sizeof(data)-1);close(fd);
    if(size<=0 || (size_t)size>=sizeof(data)-1)return -1;
    data[size]=0;char *end=strrchr(data,')');if(!end || end[1]!=' ')return -1;
    char *save=NULL;char *item=strtok_r(end+2," ",&save);
    for(int field=3;field<=22;field++){
        if(!item)return -1;
        if(field==22)return number(item,result);
        item=strtok_r(NULL," ",&save);
    }
    return -1;
}

static int quarantine(const struct permit *permit) {
    uint64_t observed=0;
    if(start_ticks(permit->pid,&observed)!=0 || observed!=permit->start_ticks)return -1;
    if(ptrace(PTRACE_SEIZE,permit->pid,NULL,NULL)!=0)return -1;
    int attached=1,stopped=0,status=0,ret=-1;
    if(ptrace(PTRACE_INTERRUPT,permit->pid,NULL,NULL)!=0)goto done;
    pid_t waited;
    do{waited=waitpid(permit->pid,&status,__WALL);}while(waited<0 && errno==EINTR);
    if(waited!=permit->pid || !WIFSTOPPED(status))goto done;
    stopped=1;
    if(start_ticks(permit->pid,&observed)!=0 || observed!=permit->start_ticks)goto done;
    printf("OCAF039_GUEST_QUARANTINE_STOPPED PID=%ld START=%" PRIu64 "\n",
           (long)permit->pid,observed);fflush(stdout);
    struct timespec delay={.tv_sec=(time_t)(permit->hold_ms/1000),
                           .tv_nsec=(long)(permit->hold_ms%1000)*1000000L};
    while(nanosleep(&delay,&delay)!=0 && errno==EINTR){}
    ret=0;
done:
    if(attached && ptrace(PTRACE_DETACH,permit->pid,NULL,NULL)!=0)ret=-1;
    if(ret==0 && stopped){
        printf("OCAF039_GUEST_QUARANTINE_RELEASED PID=%ld\n",(long)permit->pid);
        fflush(stdout);
    }
    return ret;
}

int main(int argc,char **argv) {
    if(argc!=3){fputs("usage: ocaf-guest-quarantine PERMIT SIGNATURE\n",stderr);return 2;}
    char data[MAX_PERMIT];struct permit permit={0};
    int len=signed_permit(argv[1],argv[2],data,sizeof(data));
    if(len<=0 || parse_permit(data,(size_t)len,&permit)!=0){
        fputs("OCAF039_GUEST_QUARANTINE_DENIED_AUTHORITY\n",stderr);return 3;
    }
    if(quarantine(&permit)!=0){
        fputs("OCAF039_GUEST_QUARANTINE_DENIED_TARGET\n",stderr);return 4;
    }
    return 0;
}
