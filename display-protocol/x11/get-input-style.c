#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/Xresource.h>
#include <X11/Xlocale.h>
#include <stdio.h>

int main() {
    // setlocale(LC_CTYPE, "");

    // Set locale modifiers. This tells XIM which input method to use.
    // XMODIFIERS is the environment variable that usually contains this.
    XSetLocaleModifiers("");
    Display *display = XOpenDisplay(NULL);
    if (!display) {
        printf("Cannot open display\n");
        return 1;
    }
    XIM xim = XOpenIM(display, NULL, NULL, NULL);
    if (!xim) {
        printf("Cannot open input method\n");
        return 1;
    }
    XIMStyles *styles = NULL;
    char *ret = XGetIMValues(xim, XNQueryInputStyle, &styles, NULL);
    if (ret) {
        printf("XGetIMValues error: %s\n", ret);
    } else {
        printf("Supported styles count: %d\n", styles->count_styles);
        for (unsigned short i = 0; i < styles->count_styles; ++i) {
            printf("Style: 0x%lx\n", styles->supported_styles[i]);
        }
        XFree(styles);
    }
    XCloseIM(xim);
    XCloseDisplay(display);
    return 0;
}
