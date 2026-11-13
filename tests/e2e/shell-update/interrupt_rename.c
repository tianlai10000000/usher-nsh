/* Pause an owned installation rename to inspect crash and architecture boundaries safely. */
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int interrupt_rename(const char *from, const char *to) {
    /* dyld does not interpose references originating in the interposing image. */
    int result = rename(from, to);
    const char *owned = getenv("USHER_ACCEPTANCE_INSTALLATION");
    const char *checkpoint = getenv("USHER_ACCEPTANCE_CHECKPOINT");
    const char *after_install = getenv("USHER_ACCEPTANCE_AFTER_INSTALL");
    if (result == 0 && owned && checkpoint &&
        strcmp(after_install ? to : from, owned) == 0) {
        int fd = open(checkpoint, O_WRONLY | O_CREAT | O_EXCL, 0600);
        if (fd < 0) _exit(126);
        size_t length = strlen(to);
        if (write(fd, to, length) != (ssize_t)length || fsync(fd) != 0) _exit(125);
        close(fd);
        raise(SIGSTOP);
    }
    return result;
}

__attribute__((used)) static struct {
    const void *replacement;
    const void *original;
} interpose __attribute__((section("__DATA,__interpose"))) = {
    (const void *)interrupt_rename, (const void *)rename
};
