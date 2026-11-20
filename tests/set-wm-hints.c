#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <errno.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    errno = 0;
    unsigned long id = strtoul(argv[1], NULL, 0);
    if (errno != 0 || id == 0) return 2;

    Display *display = XOpenDisplay(NULL);
    if (display == NULL) return 3;

    XWMHints hints = {0};
    hints.flags = XUrgencyHint;
    int result = XSetWMHints(display, (Window)id, &hints);
    XSync(display, False);
    XCloseDisplay(display);
    return result ? 0 : 4;
}
