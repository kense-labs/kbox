/* Native guest attacks: writable mounts must still meet the host write boundary. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

static unsigned denials;
static void denied(const char *name, int result) {
    if (result != -1 || (errno != EPERM && errno != EACCES)) {
        fprintf(stderr, "%s result=%d errno=%d\n", name, result, errno);
        assert(0 && "host write denial missing");
    }
    ++denials;
}

int main(void) {
    FILE *mounts = fopen("/proc/mounts", "r");
    assert(mounts);
    char line[4096], source[1024] = "", target[1024], kind[64], candidate[1024];
    while (fgets(line, sizeof(line), mounts)) {
        if (sscanf(line, "%1023s %1023s %63s", candidate, target, kind) == 3 && !strcmp(target, "/readonly"))
            strcpy(source, candidate);
    }
    fclose(mounts);
    assert(source[0]);
    assert(mount(NULL, "/readonly", NULL, MS_REMOUNT, NULL) == 0);
    assert(mkdir("/alternate", 0755) == 0 || errno == EEXIST);
    assert(mount(source, "/alternate", "virtiofs", 0, NULL) == 0);
    denied("create", open("/readonly/new", O_CREAT | O_WRONLY, 0644));
    denied("overwrite", open("/readonly/input", O_WRONLY));
    denied("append", open("/readonly/input", O_WRONLY | O_APPEND));
    denied("truncate", truncate("/readonly/input", 0));
    denied("unlink", unlink("/readonly/input"));
    denied("mkdir", mkdir("/readonly/newdir", 0755));
    denied("rmdir", rmdir("/readonly/empty"));
    denied("rename", rename("/readonly/input", "/readonly/moved"));
    denied("replace", rename("/readonly/replacement", "/readonly/input"));
    denied("symlink-create", symlink("/writable/output", "/readonly/link"));
    unlink("/writable/inward");
    assert(symlink("/readonly/input", "/writable/inward") == 0);
    denied("symlink-follow", open("/writable/inward", O_WRONLY | O_TRUNC));
    unlink("/writable/indir");
    assert(symlink("/readonly", "/writable/indir") == 0);
    denied("directory-symlink", open("/writable/indir/input", O_WRONLY | O_TRUNC));
    denied("hardlink", link("/readonly/input", "/readonly/alias"));
    denied("fresh-writable-mount", open("/alternate/input", O_WRONLY | O_TRUNC));
    pid_t child = fork();
    assert(child >= 0);
    if (!child) {
        int result = open("/readonly/child", O_CREAT | O_WRONLY, 0644);
        _exit(result == -1 && (errno == EPERM || errno == EACCES) ? 0 : 1);
    }
    int status;
    assert(waitpid(child, &status, 0) == child && WIFEXITED(status) && WEXITSTATUS(status) == 0);
    int writable = open("/writable/output", O_CREAT | O_WRONLY | O_TRUNC, 0644);
    assert(writable >= 0 && write(writable, "writable", 8) == 8);
    close(writable);
    assert(denials == 14);
    assert(umount("/alternate") == 0);
    printf("{\"status\":\"PASS\",\"hostDenials\":%u,\"guestChildDenied\":true,\"writableRemount\":true,\"freshWritableMount\":true}\n", denials);
    return 0;
}
