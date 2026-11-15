# 1 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c"
# 1 "<built-in>" 1
# 1 "<built-in>" 3
# 420 "<built-in>" 3
# 1 "<command line>" 1
# 1 "<built-in>" 2
# 1 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2




# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/assert.h" 1 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/features.h" 1 3
# 2 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/assert.h" 2 3
# 19 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/assert.h" 3
_Noreturn void __assert_fail (const char *, const char *, int, const char *);
# 6 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/errno.h" 1 3
# 10 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/errno.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/bits/errno.h" 1 3
# 11 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/errno.h" 2 3


__attribute__((const))

int *__errno_location(void);
# 7 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/fcntl.h" 1 3
# 20 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/fcntl.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 160 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef unsigned mode_t;
# 170 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef long off_t;
# 243 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef int pid_t;
# 21 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/fcntl.h" 2 3

# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/fcntl.h" 1 3
# 23 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/fcntl.h" 2 3

struct flock {
 short l_type;
 short l_whence;
 off_t l_start;
 off_t l_len;
 pid_t l_pid;
};

int creat(const char *, mode_t);
int fcntl(int, int, ...);
int open(const char *, int, ...);
int openat(int, const char *, int, ...);
int posix_fadvise(int, off_t, off_t, int);
int posix_fallocate(int, off_t, off_t);
# 159 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/fcntl.h" 3
int lockf(int, int, off_t);
# 8 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/stdio.h" 1 3
# 26 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/stdio.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 58 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef unsigned long size_t;
# 73 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef long ssize_t;
# 328 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef struct _IO_FILE FILE;





typedef __builtin_va_list va_list;




typedef __builtin_va_list __isoc_va_list;
# 27 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/stdio.h" 2 3
# 56 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/stdio.h" 3
typedef union _G_fpos64_t {
 char __opaque[16];
 long long __lldata;
 double __align;
} fpos_t;

extern FILE *const stdin;
extern FILE *const stdout;
extern FILE *const stderr;





FILE *fopen(const char *restrict, const char *restrict);
FILE *freopen(const char *restrict, const char *restrict, FILE *restrict);
int fclose(FILE *);

int remove(const char *);
int rename(const char *, const char *);

int feof(FILE *);
int ferror(FILE *);
int fflush(FILE *);
void clearerr(FILE *);

int fseek(FILE *, long, int);
long ftell(FILE *);
void rewind(FILE *);

int fgetpos(FILE *restrict, fpos_t *restrict);
int fsetpos(FILE *, const fpos_t *);

size_t fread(void *restrict, size_t, size_t, FILE *restrict);
size_t fwrite(const void *restrict, size_t, size_t, FILE *restrict);

int fgetc(FILE *);
int getc(FILE *);
int getchar(void);
int ungetc(int, FILE *);

int fputc(int, FILE *);
int putc(int, FILE *);
int putchar(int);

char *fgets(char *restrict, int, FILE *restrict);




int fputs(const char *restrict, FILE *restrict);
int puts(const char *);

int printf(const char *restrict, ...);
int fprintf(FILE *restrict, const char *restrict, ...);
int sprintf(char *restrict, const char *restrict, ...);
int snprintf(char *restrict, size_t, const char *restrict, ...);

int vprintf(const char *restrict, __isoc_va_list);
int vfprintf(FILE *restrict, const char *restrict, __isoc_va_list);
int vsprintf(char *restrict, const char *restrict, __isoc_va_list);
int vsnprintf(char *restrict, size_t, const char *restrict, __isoc_va_list);

int scanf(const char *restrict, ...);
int fscanf(FILE *restrict, const char *restrict, ...);
int sscanf(const char *restrict, const char *restrict, ...);
int vscanf(const char *restrict, __isoc_va_list);
int vfscanf(FILE *restrict, const char *restrict, __isoc_va_list);
int vsscanf(const char *restrict, const char *restrict, __isoc_va_list);

void perror(const char *);

int setvbuf(FILE *restrict, char *restrict, int, size_t);
void setbuf(FILE *restrict, char *restrict);

char *tmpnam(char *);
FILE *tmpfile(void);




FILE *fmemopen(void *restrict, size_t, const char *restrict);
FILE *open_memstream(char **, size_t *);
FILE *fdopen(int, const char *);
FILE *popen(const char *, const char *);
int pclose(FILE *);
int fileno(FILE *);
int fseeko(FILE *, off_t, int);
off_t ftello(FILE *);
int dprintf(int, const char *restrict, ...);
int vdprintf(int, const char *restrict, __isoc_va_list);
void flockfile(FILE *);
int ftrylockfile(FILE *);
void funlockfile(FILE *);
int getc_unlocked(FILE *);
int getchar_unlocked(void);
int putc_unlocked(int, FILE *);
int putchar_unlocked(int);
ssize_t getdelim(char **restrict, size_t *restrict, int, FILE *restrict);
ssize_t getline(char **restrict, size_t *restrict, FILE *restrict);
int renameat(int, const char *, int, const char *);
char *ctermid(char *);







char *tempnam(const char *, const char *);




char *cuserid(char *);
void setlinebuf(FILE *);
void setbuffer(FILE *, char *, size_t);
int fgetc_unlocked(FILE *);
int fputc_unlocked(int, FILE *);
int fflush_unlocked(FILE *);
size_t fread_unlocked(void *, size_t, size_t, FILE *);
size_t fwrite_unlocked(const void *, size_t, size_t, FILE *);
void clearerr_unlocked(FILE *);
int feof_unlocked(FILE *);
int ferror_unlocked(FILE *);
int fileno_unlocked(FILE *);
int getw(FILE *);
int putw(int, FILE *);
char *fgetln(FILE *, size_t *);
int asprintf(char **, const char *, ...);
int vasprintf(char **, const char *, __isoc_va_list);
# 9 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/string.h" 1 3
# 25 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/string.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 351 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef struct __locale_struct * locale_t;
# 26 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/string.h" 2 3

void *memcpy (void *restrict, const void *restrict, size_t);
void *memmove (void *, const void *, size_t);
void *memset (void *, int, size_t);
int memcmp (const void *, const void *, size_t);
void *memchr (const void *, int, size_t);

char *strcpy (char *restrict, const char *restrict);
char *strncpy (char *restrict, const char *restrict, size_t);

char *strcat (char *restrict, const char *restrict);
char *strncat (char *restrict, const char *restrict, size_t);

int strcmp (const char *, const char *);
int strncmp (const char *, const char *, size_t);

int strcoll (const char *, const char *);
size_t strxfrm (char *restrict, const char *restrict, size_t);

char *strchr (const char *, int);
char *strrchr (const char *, int);

size_t strcspn (const char *, const char *);
size_t strspn (const char *, const char *);
char *strpbrk (const char *, const char *);
char *strstr (const char *, const char *);
char *strtok (char *restrict, const char *restrict);

size_t strlen (const char *);

char *strerror (int);


# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/strings.h" 1 3
# 12 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/strings.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 13 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/strings.h" 2 3




int bcmp (const void *, const void *, size_t);
void bcopy (const void *, void *, size_t);
void bzero (void *, size_t);
char *index (const char *, int);
char *rindex (const char *, int);



int ffs (int);
int ffsl (long);
int ffsll (long long);


int strcasecmp (const char *, const char *);
int strncasecmp (const char *, const char *, size_t);

int strcasecmp_l (const char *, const char *, locale_t);
int strncasecmp_l (const char *, const char *, size_t, locale_t);
# 60 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/string.h" 2 3





char *strtok_r (char *restrict, const char *restrict, char **restrict);
int strerror_r (int, char *, size_t);
char *stpcpy(char *restrict, const char *restrict);
char *stpncpy(char *restrict, const char *restrict, size_t);
size_t strnlen (const char *, size_t);
char *strdup (const char *);
char *strndup (const char *, size_t);
char *strsignal(int);
char *strerror_l (int, locale_t);
int strcoll_l (const char *, const char *, locale_t);
size_t strxfrm_l (char *restrict, const char *restrict, size_t, locale_t);
void *memmem(const void *, size_t, const void *, size_t);




void *memccpy (void *restrict, const void *restrict, int, size_t);



char *strsep(char **, const char *);
size_t strlcat (char *, const char *, size_t);
size_t strlcpy (char *, const char *, size_t);
void explicit_bzero (void *, size_t);
# 10 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/mount.h" 1 3







# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/ioctl.h" 1 3








# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 369 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
struct winsize { unsigned short ws_row, ws_col, ws_xpixel, ws_ypixel; };
# 10 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/ioctl.h" 2 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/bits/ioctl.h" 1 3
# 115 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/bits/ioctl.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/bits/ioctl_fix.h" 1 3
# 115 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/bits/ioctl.h" 2 3
# 11 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/ioctl.h" 2 3
# 115 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/ioctl.h" 3
int ioctl (int, int, ...);
# 9 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/mount.h" 2 3
# 67 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/mount.h" 3
int mount(const char *, const char *, const char *, unsigned long, const void *);
int umount(const char *);
int umount2(const char *, int);
# 11 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/stat.h" 1 3
# 28 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/stat.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 27 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef int blksize_t;




typedef unsigned int nlink_t;
# 93 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef long time_t;
# 175 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef unsigned long ino_t;




typedef unsigned long dev_t;
# 190 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef long blkcnt_t;
# 237 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
struct timespec { time_t tv_sec; int :8*(sizeof(time_t)-sizeof(long))*(1234==4321); long tv_nsec; int :8*(sizeof(time_t)-sizeof(long))*(1234!=4321); };
# 253 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef unsigned uid_t;




typedef unsigned gid_t;
# 29 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/stat.h" 2 3

# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/bits/stat.h" 1 3
struct stat {
 dev_t st_dev;
 ino_t st_ino;
 mode_t st_mode;
 nlink_t st_nlink;
 uid_t st_uid;
 gid_t st_gid;
 dev_t st_rdev;
 unsigned long __pad;
 off_t st_size;
 blksize_t st_blksize;
 int __pad2;
 blkcnt_t st_blocks;
 struct timespec st_atim;
 struct timespec st_mtim;
 struct timespec st_ctim;
 unsigned __unused[2];
};
# 31 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/stat.h" 2 3
# 80 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/stat.h" 3
int stat(const char *restrict, struct stat *restrict);
int fstat(int, struct stat *);
int lstat(const char *restrict, struct stat *restrict);
int fstatat(int, const char *restrict, struct stat *restrict, int);
int chmod(const char *, mode_t);
int fchmod(int, mode_t);
int fchmodat(int, const char *, mode_t, int);
mode_t umask(mode_t);
int mkdir(const char *, mode_t);
int mkfifo(const char *, mode_t);
int mkdirat(int, const char *, mode_t);
int mkfifoat(int, const char *, mode_t);


int mknod(const char *, mode_t, dev_t);
int mknodat(int, const char *, mode_t, dev_t);


int futimens(int, const struct timespec [2]);
int utimensat(int, const char *, const struct timespec [2], int);


int lchmod(const char *, mode_t);
# 12 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/wait.h" 1 3
# 11 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/wait.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 248 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef unsigned id_t;
# 12 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/wait.h" 2 3

typedef enum {
 P_ALL = 0,
 P_PID = 1,
 P_PGID = 2,
 P_PIDFD = 3
} idtype_t;

pid_t wait (int *);
pid_t waitpid (pid_t, int *, int );




# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 1 3
# 28 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 227 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef long clock_t;
# 281 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef struct __pthread * pthread_t;
# 357 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef struct __sigset_t { unsigned long __bits[128/sizeof(long)]; } sigset_t;
# 386 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef struct { union { int __i[sizeof(long)==8?14:9]; volatile int __vi[sizeof(long)==8?14:9]; unsigned long __s[sizeof(long)==8?7:9]; } __u; } pthread_attr_t;
# 29 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 2 3
# 44 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 3
typedef struct sigaltstack stack_t;



# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/signal.h" 1 3
# 10 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/signal.h" 3
typedef unsigned long greg_t;
typedef unsigned long gregset_t[34];

typedef struct {
 __uint128_t vregs[32];
 unsigned int fpsr;
 unsigned int fpcr;
} fpregset_t;
typedef struct sigcontext {
 unsigned long fault_address;
 unsigned long regs[31];
 unsigned long sp, pc, pstate;
 long double __reserved[256];
} mcontext_t;





struct _aarch64_ctx {
 unsigned int magic;
 unsigned int size;
};
struct fpsimd_context {
 struct _aarch64_ctx head;
 unsigned int fpsr;
 unsigned int fpcr;
 __uint128_t vregs[32];
};
struct esr_context {
 struct _aarch64_ctx head;
 unsigned long esr;
};
struct extra_context {
 struct _aarch64_ctx head;
 unsigned long datap;
 unsigned int size;
 unsigned int __reserved[3];
};
struct sve_context {
 struct _aarch64_ctx head;
 unsigned short vl;
 unsigned short __reserved[3];
};
# 93 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/signal.h" 3
struct sigaltstack {
 void *ss_sp;
 int ss_flags;
 size_t ss_size;
};

typedef struct __ucontext {
 unsigned long uc_flags;
 struct __ucontext *uc_link;
 stack_t uc_stack;
 sigset_t uc_sigmask;
 mcontext_t uc_mcontext;
} ucontext_t;
# 49 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 2 3
# 94 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 3
union sigval {
 int sival_int;
 void *sival_ptr;
};

typedef struct {



 int si_signo, si_errno, si_code;

 union {
  char __pad[128 - 2*sizeof(int) - sizeof(long)];
  struct {
   union {
    struct {
     pid_t si_pid;
     uid_t si_uid;
    } __piduid;
    struct {
     int si_timerid;
     int si_overrun;
    } __timer;
   } __first;
   union {
    union sigval si_value;
    struct {
     int si_status;
     clock_t si_utime, si_stime;
    } __sigchld;
   } __second;
  } __si_common;
  struct {
   void *si_addr;
   short si_addr_lsb;
   union {
    struct {
     void *si_lower;
     void *si_upper;
    } __addr_bnd;
    unsigned si_pkey;
   } __first;
  } __sigfault;
  struct {
   long si_band;
   int si_fd;
  } __sigpoll;
  struct {
   void *si_call_addr;
   int si_syscall;
   unsigned si_arch;
  } __sigsys;
 } __si_fields;
} siginfo_t;
# 169 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 3
struct sigaction {
 union {
  void (*sa_handler)(int);
  void (*sa_sigaction)(int, siginfo_t *, void *);
 } __sa_handler;
 sigset_t sa_mask;
 int sa_flags;
 void (*sa_restorer)(void);
};






struct sigevent {
 union sigval sigev_value;
 int sigev_signo;
 int sigev_notify;
 union {
  char __pad[64 - 2*sizeof(int) - sizeof(union sigval)];
  pid_t sigev_notify_thread_id;
  struct {
   void (*sigev_notify_function)(union sigval);
   pthread_attr_t *sigev_notify_attributes;
  } __sev_thread;
 } __sev_fields;
};
# 207 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 3
int __libc_current_sigrtmin(void);
int __libc_current_sigrtmax(void);




int kill(pid_t, int);

int sigemptyset(sigset_t *);
int sigfillset(sigset_t *);
int sigaddset(sigset_t *, int);
int sigdelset(sigset_t *, int);
int sigismember(const sigset_t *, int);

int sigprocmask(int, const sigset_t *restrict, sigset_t *restrict);
int sigsuspend(const sigset_t *);
int sigaction(int, const struct sigaction *restrict, struct sigaction *restrict);
int sigpending(sigset_t *);
int sigwait(const sigset_t *restrict, int *restrict);
int sigwaitinfo(const sigset_t *restrict, siginfo_t *restrict);
int sigtimedwait(const sigset_t *restrict, siginfo_t *restrict, const struct timespec *restrict);
int sigqueue(pid_t, int, union sigval);

int pthread_sigmask(int, const sigset_t *restrict, sigset_t *restrict);
int pthread_kill(pthread_t, int);

void psiginfo(const siginfo_t *, const char *);
void psignal(int, const char *);




int killpg(pid_t, int);
int sigaltstack(const stack_t *restrict, stack_t *restrict);
int sighold(int);
int sigignore(int);
int siginterrupt(int, int);
int sigpause(int);
int sigrelse(int);
void (*sigset(int, void (*)(int)))(int);
# 266 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 3
typedef void (*sig_t)(int);
# 287 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/signal.h" 3
typedef int sig_atomic_t;

void (*signal(int, void (*)(int)))(int);
int raise(int);
# 27 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/wait.h" 2 3
int waitid (idtype_t, id_t, siginfo_t *, int);



# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/resource.h" 1 3








# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/time.h" 1 3








# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/select.h" 1 3
# 16 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/select.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 98 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef long suseconds_t;
# 232 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
struct timeval { time_t tv_sec; suseconds_t tv_usec; };
# 17 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/select.h" 2 3



typedef unsigned long fd_mask;

typedef struct {
 unsigned long fds_bits[1024 / 8 / sizeof(long)];
} fd_set;






int select (int, fd_set *restrict, fd_set *restrict, fd_set *restrict, struct timeval *restrict);
int pselect (int, fd_set *restrict, fd_set *restrict, fd_set *restrict, const struct timespec *restrict, const sigset_t *restrict);
# 10 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/time.h" 2 3

int gettimeofday (struct timeval *restrict, void *restrict);





struct itimerval {
 struct timeval it_interval;
 struct timeval it_value;
};

int getitimer (int, struct itimerval *);
int setitimer (int, const struct itimerval *restrict, struct itimerval *restrict);
int utimes (const char *, const struct timeval [2]);


struct timezone {
 int tz_minuteswest;
 int tz_dsttime;
};
int futimes(int, const struct timeval [2]);
int futimesat(int, const char *, const struct timeval [2]);
int lutimes(const char *, const struct timeval [2]);
int settimeofday(const struct timeval *, const struct timezone *);
int adjtime (const struct timeval *, struct timeval *);
# 10 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/resource.h" 2 3







# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 18 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/resource.h" 2 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/bits/resource.h" 1 3
# 19 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/resource.h" 2 3

typedef unsigned long long rlim_t;

struct rlimit {
 rlim_t rlim_cur;
 rlim_t rlim_max;
};

struct rusage {
 struct timeval ru_utime;
 struct timeval ru_stime;

 long ru_maxrss;
 long ru_ixrss;
 long ru_idrss;
 long ru_isrss;
 long ru_minflt;
 long ru_majflt;
 long ru_nswap;
 long ru_inblock;
 long ru_oublock;
 long ru_msgsnd;
 long ru_msgrcv;
 long ru_nsignals;
 long ru_nvcsw;
 long ru_nivcsw;

 long __reserved[16];
};

int getrlimit (int, struct rlimit *);
int setrlimit (int, const struct rlimit *);
int getrusage (int, struct rusage *);

int getpriority (int, id_t);
int setpriority (int, id_t, int);
# 32 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/sys/wait.h" 2 3
pid_t wait3 (int *, int, struct rusage *);
pid_t wait4 (pid_t, int *, int, struct rusage *);
# 13 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/unistd.h" 1 3
# 37 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/unistd.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 1 3
# 78 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef long intptr_t;
# 268 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/alltypes.h" 3
typedef unsigned useconds_t;
# 38 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/unistd.h" 2 3

int pipe(int [2]);
int pipe2(int [2], int);
int close(int);
int posix_close(int, int);
int dup(int);
int dup2(int, int);
int dup3(int, int, int);
off_t lseek(int, off_t, int);
int fsync(int);
int fdatasync(int);

ssize_t read(int, void *, size_t);
ssize_t write(int, const void *, size_t);
ssize_t pread(int, void *, size_t, off_t);
ssize_t pwrite(int, const void *, size_t, off_t);

int chown(const char *, uid_t, gid_t);
int fchown(int, uid_t, gid_t);
int lchown(const char *, uid_t, gid_t);
int fchownat(int, const char *, uid_t, gid_t, int);

int link(const char *, const char *);
int linkat(int, const char *, int, const char *, int);
int symlink(const char *, const char *);
int symlinkat(const char *, int, const char *);
ssize_t readlink(const char *restrict, char *restrict, size_t);
ssize_t readlinkat(int, const char *restrict, char *restrict, size_t);
int unlink(const char *);
int unlinkat(int, const char *, int);
int rmdir(const char *);
int truncate(const char *, off_t);
int ftruncate(int, off_t);






int access(const char *, int);
int faccessat(int, const char *, int, int);

int chdir(const char *);
int fchdir(int);
char *getcwd(char *, size_t);

unsigned alarm(unsigned);
unsigned sleep(unsigned);
int pause(void);

pid_t fork(void);
pid_t _Fork(void);
int execve(const char *, char *const [], char *const []);
int execv(const char *, char *const []);
int execle(const char *, const char *, ...);
int execl(const char *, const char *, ...);
int execvp(const char *, char *const []);
int execlp(const char *, const char *, ...);
int fexecve(int, char *const [], char *const []);
_Noreturn void _exit(int);

pid_t getpid(void);
pid_t getppid(void);
pid_t getpgrp(void);
pid_t getpgid(pid_t);
int setpgid(pid_t, pid_t);
pid_t setsid(void);
pid_t getsid(pid_t);
char *ttyname(int);
int ttyname_r(int, char *, size_t);
int isatty(int);
pid_t tcgetpgrp(int);
int tcsetpgrp(int, pid_t);

uid_t getuid(void);
uid_t geteuid(void);
gid_t getgid(void);
gid_t getegid(void);
int getgroups(int, gid_t []);
int setuid(uid_t);
int seteuid(uid_t);
int setgid(gid_t);
int setegid(gid_t);

char *getlogin(void);
int getlogin_r(char *, size_t);
int gethostname(char *, size_t);
char *ctermid(char *);

int getopt(int, char * const [], const char *);
extern char *optarg;
extern int optind, opterr, optopt;

long pathconf(const char *, int);
long fpathconf(int, int);
long sysconf(int);
size_t confstr(int, char *, size_t);






int setreuid(uid_t, uid_t);
int setregid(gid_t, gid_t);
int lockf(int, int, off_t);
long gethostid(void);
int nice(int);
void sync(void);
pid_t setpgrp(void);
char *crypt(const char *, const char *);
void encrypt(char *, int);
void swab(const void *restrict, void *restrict, ssize_t);




int usleep(unsigned);
unsigned ualarm(unsigned, unsigned);






int brk(void *);
void *sbrk(intptr_t);
pid_t vfork(void);
int vhangup(void);
int chroot(const char *);
int getpagesize(void);
int getdtablesize(void);
int sethostname(const char *, size_t);
int getdomainname(char *, size_t);
int setdomainname(const char *, size_t);
int setgroups(size_t, const gid_t *);
char *getpass(const char *);
int daemon(int, int);
void setusershell(void);
void endusershell(void);
char *getusershell(void);
int acct(const char *);
long syscall(long, ...);
int execvpe(const char *, char *const [], char *const []);
int issetugid(void);
int getentropy(void *, size_t);
extern int optreset;
# 260 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/unistd.h" 3
# 1 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/aarch64-linux-musl/bits/posix.h" 1 3
# 261 "/opt/homebrew/Cellar/zig/0.16.0_1/lib/zig/libc/include/generic-musl/unistd.h" 2 3
# 14 "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c" 2

static unsigned denials;
static void denied(const char *name, int result) {
    if (result != -1 || ((*__errno_location()) != 1 && (*__errno_location()) != 13)) {
        fprintf((stderr), "%s result=%d errno=%d\n", name, result, (*__errno_location()));
        ((void)((0 && "host write denial missing") || (__assert_fail("0 && \"host write denial missing\"", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 19, __func__),0)));
    }
    ++denials;
}

int main(void) {
    FILE *mounts = fopen("/proc/mounts", "r");
    ((void)((mounts) || (__assert_fail("mounts", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 26, __func__),0)));
    char line[4096], source[1024] = "", target[1024], kind[64], candidate[1024];
    while (fgets(line, sizeof(line), mounts)) {
        if (sscanf(line, "%1023s %1023s %63s", candidate, target, kind) == 3 && !strcmp(target, "/readonly"))
            strcpy(source, candidate);
    }
    fclose(mounts);
    ((void)((source[0]) || (__assert_fail("source[0]", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 33, __func__),0)));
    ((void)((mount(((void*)0), "/readonly", ((void*)0), 32, ((void*)0)) == 0) || (__assert_fail("mount(NULL, \"/readonly\", NULL, MS_REMOUNT, NULL) == 0", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 34, __func__),0)));
    ((void)((mkdir("/alternate", 0755) == 0 || (*__errno_location()) == 17) || (__assert_fail("mkdir(\"/alternate\", 0755) == 0 || errno == EEXIST", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 35, __func__),0)));
    ((void)((mount(source, "/alternate", "virtiofs", 0, ((void*)0)) == 0) || (__assert_fail("mount(source, \"/alternate\", \"virtiofs\", 0, NULL) == 0", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 36, __func__),0)));
    denied("create", open("/readonly/new", 0100 | 01, 0644));
    denied("overwrite", open("/readonly/input", 01));
    denied("append", open("/readonly/input", 01 | 02000));
    denied("truncate", truncate("/readonly/input", 0));
    denied("unlink", unlink("/readonly/input"));
    denied("mkdir", mkdir("/readonly/newdir", 0755));
    denied("rmdir", rmdir("/readonly/empty"));
    denied("rename", rename("/readonly/input", "/readonly/moved"));
    denied("replace", rename("/readonly/replacement", "/readonly/input"));
    denied("symlink-create", symlink("/writable/output", "/readonly/link"));
    unlink("/writable/inward");
    ((void)((symlink("/readonly/input", "/writable/inward") == 0) || (__assert_fail("symlink(\"/readonly/input\", \"/writable/inward\") == 0", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 48, __func__),0)));
    denied("symlink-follow", open("/writable/inward", 01 | 01000));
    unlink("/writable/indir");
    ((void)((symlink("/readonly", "/writable/indir") == 0) || (__assert_fail("symlink(\"/readonly\", \"/writable/indir\") == 0", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 51, __func__),0)));
    denied("directory-symlink", open("/writable/indir/input", 01 | 01000));
    denied("hardlink", link("/readonly/input", "/readonly/alias"));
    denied("fresh-writable-mount", open("/alternate/input", 01 | 01000));
    pid_t child = fork();
    ((void)((child >= 0) || (__assert_fail("child >= 0", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 56, __func__),0)));
    if (!child) {
        int result = open("/readonly/child", 0100 | 01, 0644);
        _exit(result == -1 && ((*__errno_location()) == 1 || (*__errno_location()) == 13) ? 0 : 1);
    }
    int status;
    ((void)((waitpid(child, &status, 0) == child && (!((status) & 0x7f)) && (((status) & 0xff00) >> 8) == 0) || (__assert_fail("waitpid(child, &status, 0) == child && WIFEXITED(status) && WEXITSTATUS(status) == 0", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 62, __func__),0)));
    int writable = open("/writable/output", 0100 | 01 | 01000, 0644);
    ((void)((writable >= 0 && write(writable, "writable", 8) == 8) || (__assert_fail("writable >= 0 && write(writable, \"writable\", 8) == 8", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 64, __func__),0)));
    close(writable);
    ((void)((denials == 14) || (__assert_fail("denials == 14", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 66, __func__),0)));
    ((void)((umount("/alternate") == 0) || (__assert_fail("umount(\"/alternate\") == 0", "/Users/roylin/code/kense/kbox/crates/box/runtime/tests/fixtures/hvf_readonly.c", 67, __func__),0)));
    printf("{\"status\":\"PASS\",\"hostDenials\":%u,\"guestChildDenied\":true,\"writableRemount\":true,\"freshWritableMount\":true}\n", denials);
    return 0;
}
