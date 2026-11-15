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
#include <sys/prctl.h>
#include <sys/mount.h>
#include <sys/statfs.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/sysmacros.h>
#include <sys/wait.h>
#include <sys/xattr.h>
#include <unistd.h>

#define BIT(n) (UINT64_C(1)<<(n))
#define BASE (BIT(CAP_CHOWN)|BIT(CAP_DAC_OVERRIDE)|BIT(CAP_FOWNER)|BIT(CAP_FSETID)|BIT(CAP_KILL)|BIT(CAP_SETGID)|BIT(CAP_SETUID)|BIT(CAP_SETPCAP)|BIT(CAP_NET_BIND_SERVICE)|BIT(CAP_SYS_CHROOT))
#define EXTRA (BIT(CAP_MKNOD)|BIT(CAP_SETFCAP)|BIT(CAP_AUDIT_WRITE))
static uint64_t expected;
static uid_t expected_uid;
static int allowed;
static uint64_t carried;
static uint64_t execution;

static void credentials(void) {
    assert(getuid()==expected_uid&&geteuid()==expected_uid);
    assert(prctl(PR_GET_NO_NEW_PRIVS,0,0,0,0)==1);
    struct __user_cap_header_struct header={_LINUX_CAPABILITY_VERSION_3,0};
    struct __user_cap_data_struct data[2];
    assert(syscall(SYS_capget,&header,data)==0);
    assert((((uint64_t)data[1].effective<<32)|data[0].effective)==execution);
    assert((((uint64_t)data[1].permitted<<32)|data[0].permitted)==execution);
    assert((((uint64_t)data[1].inheritable<<32)|data[0].inheritable)==carried);
    for(unsigned bit=0;bit<=40;bit++) {
        int value=prctl(PR_CAPBSET_READ,bit,0,0,0);assert(value>=0);
        assert(value==((expected>>bit)&1));
        assert(prctl(PR_CAP_AMBIENT,PR_CAP_AMBIENT_IS_SET,bit,0,0)==((carried>>bit)&1));
    }
    if(expected_uid!=0&&expected==0) {
        struct rlimit limit;assert(getrlimit(RLIMIT_NPROC,&limit)==0);
        assert(limit.rlim_cur==16&&limit.rlim_max==16);
    }
}

static void operations(void) {
    credentials();
    char directory[128];
    snprintf(directory,sizeof(directory),"%s/capability-XXXXXX",expected_uid==0?"":"/ownership");
    assert(mkdtemp(directory));
    char node[160],file[160];snprintf(node,sizeof(node),"%s/node",directory);snprintf(file,sizeof(file),"%s/file",directory);
    errno=0;int result=mknod(node,S_IFCHR|0600,makedev(1,3));int node_errno=result<0?errno:0;
    if((allowed&&result!=0)||(!allowed&&node_errno!=EPERM)) fprintf(stderr,"mknod uid=%u expectedAllowed=%d result=%d errno=%d\n",(unsigned)expected_uid,allowed,result,node_errno);
    if(allowed) {
        assert(result==0);struct stat metadata;assert(lstat(node,&metadata)==0);
        assert(S_ISCHR(metadata.st_mode)&&major(metadata.st_rdev)==1&&minor(metadata.st_rdev)==3);
        assert(metadata.st_uid==expected_uid);
        int fd=open(node,O_RDONLY|O_CLOEXEC);assert(fd<0&&(errno==EPERM||errno==EACCES));
        assert(unlink(node)==0);
    } else {assert(result==-1&&node_errno==EPERM);assert(access(node,F_OK)==-1&&errno==ENOENT);}
    int fd=open(file,O_RDWR|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0600);assert(fd>=0);
    struct vfs_cap_data value={0};value.magic_etc=VFS_CAP_REVISION_2|VFS_CAP_FLAGS_EFFECTIVE;value.data[0].permitted=1U<<CAP_CHOWN;
    errno=0;result=fsetxattr(fd,"security.capability",&value,sizeof(value),0);int cap_errno=result<0?errno:0;
    if((allowed&&result!=0)||(!allowed&&cap_errno!=EPERM)) fprintf(stderr,"fsetxattr uid=%u expectedAllowed=%d result=%d errno=%d\n",(unsigned)expected_uid,allowed,result,cap_errno);
    struct vfs_cap_data actual={0};
    if(allowed) {assert(result==0);assert(fgetxattr(fd,"security.capability",&actual,sizeof(actual))==(ssize_t)sizeof(actual));assert(memcmp(&value,&actual,sizeof(actual))==0);assert(fremovexattr(fd,"security.capability")==0);}
    else {assert(result==-1&&cap_errno==EPERM);assert(fgetxattr(fd,"security.capability",&actual,sizeof(actual))==-1&&errno==ENODATA);}
    assert(write(fd,"plain",5)==5);assert(fsync(fd)==0&&close(fd)==0);
    assert(unlink(file)==0);
    char mountpoint[160];snprintf(mountpoint,sizeof(mountpoint),"%s/mount",directory);assert(mkdir(mountpoint,0700)==0);
    errno=0;result=mount("tmpfs",mountpoint,"tmpfs",MS_NOSUID|MS_NODEV,"size=65536");int mount_errno=result<0?errno:0;
    if(expected&BIT(CAP_SYS_ADMIN)) {assert(result==0);struct statfs fs;assert(statfs(mountpoint,&fs)==0&&fs.f_type==0x01021994);assert(umount(mountpoint)==0);}
    else {assert(result==-1&&mount_errno==EPERM);}
    assert(rmdir(mountpoint)==0&&rmdir(directory)==0);
}

static void *thread_probe(void *unused) {(void)unused;operations();return NULL;}

int main(int argc,char **argv) {
    assert(argc==3||argc==4);
    if(!strcmp(argv[1],"added")) {expected=BASE|EXTRA;allowed=1;}
    else if(!strcmp(argv[1],"added-admin")) {expected=BASE|EXTRA|BIT(CAP_SYS_ADMIN);allowed=1;}
    else if(!strcmp(argv[1],"dropped")) {expected=BASE;allowed=0;}
    else {assert(!strcmp(argv[1],"all"));expected=0;allowed=0;}
    char *end;unsigned long uid=strtoul(argv[2],&end,10);assert(!*end&&(uid==0||uid==1001));expected_uid=(uid_t)uid;
    carried=expected_uid==0?0:expected&(EXTRA|BIT(CAP_SYS_ADMIN));
    execution=expected_uid==0?expected:carried;
    if(argc==4) {assert(!strcmp(argv[3],"--stdin"));char line[32];assert(fgets(line,sizeof(line),stdin)&&!strcmp(line,"streamed\n"));}
    operations();
    pid_t child=fork();assert(child>=0);
    if(child==0) {operations();_exit(0);}
    int status;assert(waitpid(child,&status,0)==child&&WIFEXITED(status)&&WEXITSTATUS(status)==0);
    pthread_t thread;assert(pthread_create(&thread,NULL,thread_probe,NULL)==0);assert(pthread_join(thread,NULL)==0);
    credentials();
    printf("{\"status\":\"PASS\",\"uid\":%u,\"tty\":%s,\"bounding\":\"%"PRIx64"\",\"effectivePermitted\":\"%"PRIx64"\",\"inheritableAmbient\":\"%"PRIx64"\",\"emptyInheritableAndAmbient\":%s,\"noNewPrivileges\":true,\"mknodAllowed\":%s,\"mknodErrno\":%d,\"fileCapabilityAllowed\":%s,\"fileCapabilityErrno\":%d,\"sysAdminMountAllowed\":%s,\"sysAdminMountErrno\":%d,\"plainWritesAllowed\":true,\"guestDeviceOpenDenied\":true,\"childInheritedAndReaped\":true,\"threadInheritedAndJoined\":true,\"ownedObjectsRemoved\":true,\"perUidHardLimitRetained\":%s}\n",
        (unsigned)expected_uid,isatty(1)?"true":"false",expected,execution,carried,carried==0?"true":"false",allowed?"true":"false",allowed?0:EPERM,allowed?"true":"false",allowed?0:EPERM,expected&BIT(CAP_SYS_ADMIN)?"true":"false",expected&BIT(CAP_SYS_ADMIN)?0:EPERM,expected_uid!=0&&expected==0?"true":"false");
    return 0;
}
