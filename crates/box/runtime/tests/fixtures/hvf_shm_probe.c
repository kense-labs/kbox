#define _GNU_SOURCE
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <linux/capability.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/statfs.h>
#include <sys/statvfs.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

static uint64_t requested;
static uid_t expected_uid;
static unsigned serial;

static uint64_t inspect(void) {
    assert(getuid()==expected_uid&&geteuid()==expected_uid);
    assert(prctl(PR_GET_NO_NEW_PRIVS,0,0,0,0)==1);
    struct __user_cap_header_struct h={_LINUX_CAPABILITY_VERSION_3,0};
    struct __user_cap_data_struct caps[2];assert(syscall(SYS_capget,&h,caps)==0);
    for(unsigned i=0;i<2;i++)assert(caps[i].effective==0&&caps[i].permitted==0&&caps[i].inheritable==0);
    for(unsigned bit=0;bit<=40;bit++) {
        assert(prctl(PR_CAPBSET_READ,bit,0,0,0)==0);
        assert(prctl(PR_CAP_AMBIENT,PR_CAP_AMBIENT_IS_SET,bit,0,0)==0);
    }
    struct statfs fs;assert(statfs("/dev/shm",&fs)==0&&fs.f_type==0x01021994);
    struct statvfs v;assert(statvfs("/dev/shm",&v)==0);
    uint64_t bytes=(uint64_t)v.f_blocks*v.f_frsize;
    if(requested)assert(bytes==requested);
    assert(v.f_flag&ST_NOSUID);assert(v.f_flag&ST_NODEV);assert(v.f_flag&ST_NOEXEC);
    assert(!(v.f_flag&ST_RDONLY));assert(v.f_frsize==4096);
    struct stat st;assert(lstat("/dev/shm",&st)==0&&S_ISDIR(st.st_mode));
    assert((st.st_mode&07777)==01777&&st.st_uid==0&&st.st_gid==0);
    return bytes;
}

static void reaped(pid_t child) {
    int status;assert(waitpid(child,&status,0)==child);
    assert(WIFEXITED(status)&&WEXITSTATUS(status)==0);
}

static uint64_t operations(void) {
    inspect();
    char dir[]="/dev/shm/kbox-shm-XXXXXX";assert(mkdtemp(dir));
    char name[96];snprintf(name,sizeof(name),"/kbox-shm-%ld-%u",(long)getpid(),++serial);
    int fd=shm_open(name,O_CREAT|O_EXCL|O_RDWR,0600);assert(fd>=0);
    struct stat st;assert(fstat(fd,&st)==0&&st.st_uid==expected_uid&&(st.st_mode&0777)==0600&&st.st_nlink==1);
    assert(ftruncate(fd,4096)==0);
    volatile uint64_t *shared=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);
    assert(shared!=MAP_FAILED);shared[0]=UINT64_C(0x5a1724abcddc4211);
    assert(shm_unlink(name)==0&&fstat(fd,&st)==0&&st.st_nlink==0);
    errno=0;assert(shm_open(name,O_RDONLY,0)==-1&&errno==ENOENT);
    pid_t child=fork();assert(child>=0);
    if(child==0) {assert(shared[0]==UINT64_C(0x5a1724abcddc4211));shared[0]=UINT64_C(0xc7512394);_exit(0);}
    reaped(child);assert(shared[0]==UINT64_C(0xc7512394));
    assert(munmap((void*)shared,4096)==0&&close(fd)==0);

    char path[128];snprintf(path,sizeof(path),"%s/program",dir);
    fd=open(path,O_CREAT|O_EXCL|O_WRONLY|O_NOFOLLOW|O_CLOEXEC,0755);assert(fd>=0);
    const char script[]="#!/bin/sh\nexit 0\n";assert(write(fd,script,sizeof(script)-1)==sizeof(script)-1);
    assert(close(fd)==0&&chmod(path,0755)==0);
    child=fork();assert(child>=0);
    if(child==0) {char *args[]={path,NULL},*env[]={"PATH=/bin",NULL};execve(path,args,env);_exit(errno==EACCES?0:91);}
    reaped(child);assert(unlink(path)==0);

    uint64_t written=0;
    if(requested&&requested<=2*1024*1024) {
        struct statvfs before,after;assert(statvfs("/dev/shm",&before)==0);
        snprintf(path,sizeof(path),"%s/fill",dir);
        fd=open(path,O_CREAT|O_EXCL|O_WRONLY|O_NOFOLLOW|O_CLOEXEC,0600);assert(fd>=0);
        char page[4096];memset(page,0x71,sizeof(page));
        for(;;) {
            errno=0;ssize_t n=write(fd,page,sizeof(page));
            if(n<0) {assert(errno==ENOSPC);break;}
            assert(n==(ssize_t)sizeof(page));written+=(uint64_t)n;assert(written<=requested);
        }
        assert(written>requested/2&&fsync(fd)==0&&close(fd)==0&&unlink(path)==0);
        fd=open(path,O_CREAT|O_EXCL|O_WRONLY|O_NOFOLLOW|O_CLOEXEC,0600);assert(fd>=0);
        assert(write(fd,page,sizeof(page))==sizeof(page)&&fsync(fd)==0&&close(fd)==0&&unlink(path)==0);
        assert(statvfs("/dev/shm",&after)==0&&after.f_bavail>=before.f_bavail);
    }
    assert(rmdir(dir)==0);inspect();return written;
}

static void *thread_probe(void *unused) {(void)unused;operations();return NULL;}

int main(int argc,char **argv) {
    assert(argc==3||argc==4);char *end;
    requested=strtoull(argv[1],&end,10);assert(!*end&&requested<=512*1024*1024);
    unsigned long uid=strtoul(argv[2],&end,10);assert(!*end&&(uid==0||uid==1001));expected_uid=(uid_t)uid;
    int only_inspect=argc==4&&!strcmp(argv[3],"--inspect");
    if(argc==4&&!only_inspect) {assert(!strcmp(argv[3],"--stdin"));char line[32];assert(fgets(line,sizeof(line),stdin)&&!strcmp(line,"streamed\n"));}
    uint64_t bytes=inspect(),written=0;
    if(!only_inspect) {
        written=operations();pid_t child=fork();assert(child>=0);if(child==0) {operations();_exit(0);}reaped(child);
        pthread_t thread;assert(pthread_create(&thread,NULL,thread_probe,NULL)==0&&pthread_join(thread,NULL)==0);
    }
    inspect();int full=!only_inspect&&requested&&requested<=2*1024*1024;
    printf("{\"status\":\"PASS\",\"uid\":%u,\"tty\":%s,\"bytes\":%"PRIu64",\"blockBytes\":4096,\"mode\":1023,\"ownerUid\":0,\"ownerGid\":0,\"nosuidNodevNoexec\":true,\"emptyCapabilities\":true,\"noNewPrivileges\":true,\"inspectOnly\":%s,\"posixSharedMappingAndUnlinkedFdLifetime\":%s,\"noexecErrno\":%d,\"enospcErrno\":%d,\"writtenBytes\":%"PRIu64",\"reuseSucceeded\":%s,\"childReaped\":%s,\"threadJoined\":%s,\"ownedObjectsRemoved\":true}\n",
        (unsigned)expected_uid,isatty(1)?"true":"false",bytes,only_inspect?"true":"false",only_inspect?"false":"true",only_inspect?0:EACCES,full?ENOSPC:0,written,full?"true":"false",only_inspect?"false":"true",only_inspect?"false":"true");
    return 0;
}
