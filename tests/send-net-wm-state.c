#include <X11/Xlib.h>
#include <errno.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 4) return 2;
    errno = 0;
    unsigned long id = strtoul(argv[1], NULL, 0);
    if (errno != 0 || id == 0) return 2;
    char *end = NULL;
    long action = strtol(argv[2], &end, 10);
    if (end == argv[2] || *end != '\0' || action < 0 || action > 2) return 2;

    Display *display = XOpenDisplay(NULL);
    if (display == NULL) return 3;
    Atom state = XInternAtom(display, "_NET_WM_STATE", False);
    Atom change_state = XInternAtom(display, "WM_CHANGE_STATE", False);
    Atom horizontal = XInternAtom(display, "_NET_WM_STATE_MAXIMIZED_HORZ", False);
    Atom vertical = XInternAtom(display, "_NET_WM_STATE_MAXIMIZED_VERT", False);
    Atom hidden = XInternAtom(display, "_NET_WM_STATE_HIDDEN", False);
    Atom above = XInternAtom(display, "_NET_WM_STATE_ABOVE", False);
    Atom below = XInternAtom(display, "_NET_WM_STATE_BELOW", False);

    XEvent event;
    memset(&event, 0, sizeof(event));
    event.xclient.type = ClientMessage;
    event.xclient.window = (Window)id;
    event.xclient.message_type = strcmp(argv[3], "icccm") == 0 ? change_state : state;
    event.xclient.format = 32;
    event.xclient.data.l[0] = strcmp(argv[3], "icccm") == 0 ? (action == 1 ? 3 : 1) : action;
    event.xclient.data.l[1] = strcmp(argv[3], "hidden") == 0 ? (long)hidden : strcmp(argv[3], "above") == 0 ? (long)above : strcmp(argv[3], "below") == 0 ? (long)below : (strcmp(argv[3], "vertical") == 0 ? 0 : (long)horizontal);
    event.xclient.data.l[2] = strcmp(argv[3], "hidden") == 0 || strcmp(argv[3], "above") == 0 || strcmp(argv[3], "below") == 0 || strcmp(argv[3], "horizontal") == 0 ? 0 : (long)vertical;
    event.xclient.data.l[3] = 1; /* application source indication */

    int sent = XSendEvent(display, DefaultRootWindow(display), False,
                          SubstructureRedirectMask | SubstructureNotifyMask,
                          &event);
    XSync(display, False);
    XCloseDisplay(display);
    return sent ? 0 : 4;
}
