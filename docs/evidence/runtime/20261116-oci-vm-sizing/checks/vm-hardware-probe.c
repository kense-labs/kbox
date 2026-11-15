#define _GNU_SOURCE
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint64_t cells(const unsigned char *bytes, unsigned count) {
    uint64_t value = 0;
    for (unsigned i = 0; i < count * 4; ++i) value = (value << 8) | bytes[i];
    return value;
}

static int cpu_count(const char *text) {
    unsigned count = 0, previous = 0;
    int first = 1;
    while (*text) {
        if (*text < '0' || *text > '9') return -1;
        char *end;
        unsigned long begin = strtoul(text, &end, 10), last = begin;
        if (begin > 4095 || (!first && begin <= previous)) return -1;
        if (*end == '-') {
            text = end + 1;
            if (*text < '0' || *text > '9') return -1;
            last = strtoul(text, &end, 10);
            if (last < begin || last > 4095) return -1;
        }
        count += (unsigned)(last - begin + 1);
        previous = (unsigned)last;
        first = 0;
        if (*end == '\0' || (*end == '\n' && end[1] == '\0')) return (int)count;
        if (*end != ',') return -1;
        text = end + 1;
    }
    return -1;
}

static void self_test(void) {
    assert(cpu_count("0\n") == 1);
    assert(cpu_count("0-7\n") == 8);
    assert(cpu_count("0-1,4,6-7\n") == 5);
    for (unsigned i = 0; i < 7; ++i) {
        const char *bad[] = {"", "0-", "3-1", "0-3,2-4", "0,0", "0,", "4096"};
        assert(cpu_count(bad[i]) == -1);
    }
    const unsigned char eight_gib[] = {0, 0, 0, 2, 0, 0, 0, 0};
    assert(cells(eight_gib, 2) == UINT64_C(8192) * 1024 * 1024);
}

#ifdef __linux__
#include <dirent.h>
#include <limits.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <unistd.h>

static int read_bytes(const char *path, unsigned char *bytes, size_t capacity) {
    FILE *file = fopen(path, "re");
    if (!file) return -1;
    size_t length = fread(bytes, 1, capacity, file);
    int invalid = ferror(file) || (length == capacity && fgetc(file) != EOF);
    fclose(file);
    return invalid ? -1 : (int)length;
}

static void path_join(char *path, size_t size, const char *root, const char *tail) {
    int length = snprintf(path, size, "%s/%s", root, tail);
    assert(length >= 0 && (size_t)length < size);
}

static unsigned cpu_file(const char *root, const char *name) {
    char path[PATH_MAX], text[256];
    path_join(path, sizeof(path), root, name);
    int length = read_bytes(path, (unsigned char *)text, sizeof(text) - 1);
    assert(length > 0);
    text[length] = '\0';
    int count = cpu_count(text);
    assert(count > 0);
    return (unsigned)count;
}

static uint64_t boot_memory(const char *sysfs, unsigned *nodes) {
    char root[PATH_MAX], path[PATH_MAX];
    unsigned char bytes[256];
    path_join(root, sizeof(root), sysfs, "firmware/devicetree/base");
    path_join(path, sizeof(path), root, "#address-cells");
    assert(read_bytes(path, bytes, 4) == 4);
    unsigned address_cells = (unsigned)cells(bytes, 1);
    path_join(path, sizeof(path), root, "#size-cells");
    assert(read_bytes(path, bytes, 4) == 4);
    unsigned size_cells = (unsigned)cells(bytes, 1);
    assert(address_cells >= 1 && address_cells <= 2 && size_cells >= 1 && size_cells <= 2);
    unsigned stride = (address_cells + size_cells) * 4;
    DIR *directory = opendir(root);
    assert(directory);
    uint64_t total = 0;
    struct dirent *entry;
    *nodes = 0;
    while ((entry = readdir(directory))) {
        if (entry->d_name[0] == '.') continue;
        char child[PATH_MAX];
        path_join(child, sizeof(child), root, entry->d_name);
        path_join(path, sizeof(path), child, "device_type");
        int length = read_bytes(path, bytes, sizeof(bytes));
        if (length != 7 || memcmp(bytes, "memory\0", 7)) continue;
        path_join(path, sizeof(path), child, "reg");
        length = read_bytes(path, bytes, sizeof(bytes));
        assert(length > 0 && (unsigned)length % stride == 0);
        for (unsigned offset = 0; offset < (unsigned)length; offset += stride) {
            uint64_t size = cells(bytes + offset + address_cells * 4, size_cells);
            assert(size && UINT64_MAX - total >= size);
            total += size;
        }
        assert(++*nodes <= 16);
    }
    closedir(directory);
    assert(*nodes && total);
    return total;
}

static int probe(void) {
    char root[] = "/tmp/kbox-hardware-XXXXXX";
    assert(mkdtemp(root));
    if (mount("sysfs", root, "sysfs", MS_RDONLY | MS_NODEV | MS_NOSUID | MS_NOEXEC, NULL)) {
        perror("fresh read-only Guest sysfs mount");
        rmdir(root);
        return 1;
    }
    unsigned online = cpu_file(root, "devices/system/cpu/online");
    unsigned present = cpu_file(root, "devices/system/cpu/present"), nodes;
    uint64_t memory = boot_memory(root, &nodes);
    assert(online <= present);
    assert(umount(root) == 0);
    assert(rmdir(root) == 0);
    printf("{\"onlineCpus\":%u,\"presentCpus\":%u,\"bootRamBytes\":%" PRIu64
           ",\"memoryNodes\":%u,\"freshReadonlySysfs\":true,\"guestMountRemoved\":true}\n",
           online, present, memory, nodes);
    return 0;
}
#endif

int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "--self-test") == 0) {
        self_test();
        return 0;
    }
    assert(argc == 1);
#ifdef __linux__
    return probe();
#else
    fputs("native Linux Guest required\n", stderr);
    return 2;
#endif
}
