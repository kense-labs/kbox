/* Native-only Host PEP proof: writable remounts must still meet exact data grants. */
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <sys/wait.h>
#include <unistd.h>

static int denials;
static void require(int ok, const char *operation) {
    if (!ok) { fprintf(stderr, "%s: errno=%d %s\n", operation, errno, strerror(errno)); exit(1); }
}
static void denied(int result, const char *operation) {
    int error = errno;
    require(result == -1 && (error == EPERM || error == EACCES), operation);
    denials++;
    fprintf(stderr, "HOST_DENIED %s errno=%d\n", operation, error);
}
static void tag(const char *destination, char *out) {
    FILE *mounts = fopen("/proc/mounts", "r"); require(mounts != NULL, "mount inventory");
    char source[4096], path[4096], type[64], options[4096]; int dump, pass, found=0;
    while (fscanf(mounts, "%4095s %4095s %63s %4095s %d %d\n", source,path,type,options,&dump,&pass)==6) {
        if (!strcmp(path,destination) && !strcmp(type,"virtiofs") && !strncmp(source,"kfile-",6)) {
            strcpy(out,source); found=1; break;
        }
    }
    fclose(mounts); require(found,"exact file virtiofs tag");
}
static void single_entry(const char *root) {
    DIR *d=opendir(root); require(d!=NULL,"projection enumeration"); struct dirent *e; int entries=0;
    while ((e=readdir(d))) { if (!strcmp(e->d_name,".") || !strcmp(e->d_name,"..")) continue;
        require(!strcmp(e->d_name,"file"),"caller parent or sibling leaked"); entries++; }
    closedir(d); require(entries==1,"exactly one authorized file");
}
int main(void) {
    char ro[4096],rw[4096]; tag("/exact-ro",ro);tag("/exact-rw",rw);
    require(mount(NULL,"/exact-ro",NULL,MS_BIND|MS_REMOUNT,NULL)==0,"privileged file remount rw");
    require(mkdir("/raw-ro",0755)==0 && mkdir("/raw-rw",0755)==0,"fresh mount destinations");
    require(mount(ro,"/raw-ro","virtiofs",0,NULL)==0,"fresh RO tag mounted writable");
    require(mount(rw,"/raw-rw","virtiofs",0,NULL)==0,"fresh RW tag mounted writable");
    require(mount(NULL,"/raw-ro",NULL,MS_REMOUNT,NULL)==0,"clear Guest RO superblock");
    struct statvfs flags;require(statvfs("/raw-ro",&flags)==0 && !(flags.f_flag & ST_RDONLY),"fresh RO tag actually writable in Guest");
    denied(open("/exact-ro",O_WRONLY|O_TRUNC),"remounted RO truncate");
    single_entry("/raw-ro");single_entry("/raw-rw");
    int fd=open("/raw-ro/file",O_RDONLY); require(fd>=0,"fresh RO read");close(fd);
    denied(open("/raw-ro/file",O_WRONLY),"RO modify");
    denied(open("/raw-ro/file",O_WRONLY|O_TRUNC),"RO truncate");
    denied(unlink("/raw-ro/file"),"RO unlink");
    denied(rename("/raw-ro/file","/raw-ro/moved"),"RO rename");
    denied(chmod("/raw-ro/file",0777),"RO chmod");
    denied(open("/raw-ro/sibling",O_WRONLY|O_CREAT,0666),"RO sibling create");
    denied(link("/raw-ro/file","/raw-ro/alias"),"RO hardlink");
    fd=open("/raw-rw/file",O_WRONLY|O_TRUNC);require(fd>=0,"RW data truncate");
    require(write(fd,"native-writeback",16)==16,"RW live write");close(fd);
    denied(unlink("/raw-rw/file"),"RW unlink");
    denied(rename("/raw-rw/file","/raw-rw/moved"),"RW rename");
    denied(open("/raw-rw/sibling",O_WRONLY|O_CREAT,0666),"RW sibling create");
    denied(mkdir("/raw-rw/sibling-dir",0777),"RW parent mkdir");
    denied(link("/raw-rw/file","/raw-rw/alias"),"RW hardlink");
    denied(symlink("file","/raw-rw/symlink"),"RW symlink");
    denied(chmod("/raw-rw/file",0777),"RW chmod");
    denied(chown("/raw-rw/file",0,0),"RW chown");
    pid_t child=fork();require(child>=0,"fork RO child");
    if (!child) { int w=open("/raw-ro/file",O_WRONLY|O_TRUNC);_exit(w<0 && (errno==EPERM || errno==EACCES) ? 0 : 1); }
    int status;require(waitpid(child,&status,0)==child && WIFEXITED(status) && WEXITSTATUS(status)==0,"child Host RO denial");
    require(umount("/raw-ro")==0 && umount("/raw-rw")==0,"owned fresh mounts cleanup");
    printf("{\"hostDenials\":%d,\"childDenied\":true,\"writableRemount\":true,\"freshMounts\":2,\"privateEntriesEach\":1}\n",denials);
    return 0;
}
