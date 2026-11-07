// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <termios.h>
#include <sys/ioctl.h>
#include <unistd.h>

int openpty(int *master, int *slave, char *name,
            const struct termios *termios, const struct winsize *size) {
    int (*real_openpty)(int *, int *, char *, const struct termios *,
                       const struct winsize *) = dlsym(RTLD_NEXT, "openpty");
    if (!real_openpty) { errno = ENOSYS; return -1; }
    int result = real_openpty(master, slave, name, termios, size);
    int saved_errno = errno;
    if (result == 0) {
        const char *ready = getenv("PTY_PROBE_READY");
        const char *release = getenv("PTY_PROBE_RELEASE");
        if (!ready || !release) _exit(120);
        int marker = open(ready, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
        if (marker < 0) _exit(121);
        close(marker);
        int count = 0;
        while (access(release, F_OK) != 0 && count++ < 10000) usleep(1000);
        if (access(release, F_OK) != 0) _exit(122);
    }
    errno = saved_errno;
    return result;
}
