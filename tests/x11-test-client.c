#define _POSIX_C_SOURCE 200809L
#include <X11/Xlib.h>
#include <stdio.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s WINDOW_TITLE\n", argv[0]);
        return 2;
    }

    Display *display = XOpenDisplay(NULL);
    if (display == NULL) {
        fprintf(stderr, "cannot open X11 display\n");
        return 1;
    }

    const int screen = DefaultScreen(display);
    const Window window = XCreateSimpleWindow(
        display,
        RootWindow(display, screen),
        32, 32, 640, 480, 0,
        BlackPixel(display, screen),
        WhitePixel(display, screen)
    );
    XStoreName(display, window, argv[1]);
    XSelectInput(display, window, StructureNotifyMask);
    XMapWindow(display, window);
    XSync(display, False);

    printf("0x%lx\n", (unsigned long)window);
    fflush(stdout);

    for (;;) {
        pause();
    }
}
