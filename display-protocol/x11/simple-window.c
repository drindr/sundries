#include <X11/X.h>
#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/Xos.h>
#include <X11/Xatom.h>
#include <X11/Xlocale.h>
#include <stdio.h>

int preedit_start_callback(XIM xim, XPointer client_data, XPointer call_data) {
    return -1;
}
void preedit_done_callback(XIM xim, XPointer client_data, XPointer call_data) {
}
void preedit_draw_callback(XIM xim, XPointer client_data, XIMPreeditDrawCallbackStruct *call_data) {
}
void preedit_caret_callback(XIM xim, XPointer client_data, XIMPreeditCaretCallbackStruct *call_data) {
}
int main() {
    Display *display;
    Window window;
    XIM xim;
    XIC xic;

    // New: Graphics Context and font information
    GC gc;
    XFontStruct* font_info;
    const char* text_to_display = "Hello, X11 World!";

    // Initialize the Display and XIM (same as before)
    display = XOpenDisplay(NULL);
    // ... error checking ...

    setlocale(LC_CTYPE, NULL);
    XSetLocaleModifiers("@im=fcitx5");
    xim = XOpenIM(display, NULL, NULL, NULL);
    if (!xim) {
        printf("Failed to initialize the XIM\n");
    }
    // ... error checking ...

    // Create the Window (simplified)
    window = XCreateSimpleWindow(display, DefaultRootWindow(display), 0, 0, 400, 300, 1,
                                 BlackPixel(display, DefaultScreen(display)),
                                 WhitePixel(display, DefaultScreen(display)));

    // New: Create a Graphics Context
    XGCValues gc_values;
    gc_values.foreground = BlackPixel(display, DefaultScreen(display));
    gc = XCreateGC(display, window, GCForeground, &gc_values);

    // Set up event mask to listen for KeyPress and Expose events
    XSelectInput(display, window, ExposureMask | FocusChangeMask | StructureNotifyMask);
    XMapWindow(display, window);

    // Create the Input Context (XIC)
    XVaNestedList preedit_list = XVaCreateNestedList(0,
        XNPreeditStartCallback, &preedit_start_callback,
        XNPreeditCaretCallback, &preedit_caret_callback,
        XNPreeditDoneCallback, &preedit_done_callback,
        XNPreeditDrawCallback, &preedit_draw_callback,
        NULL);
    if (!preedit_list) {
        printf("Failed to create preedit_list\n");
    }
                    //XNFocusWindow, window,
    xic = XCreateIC(xim,
                    XNInputStyle, XIMPreeditCallbacks | XIMStatusNothing,
                    XNClientWindow, window,
                    XNPreeditAttributes, preedit_list,
                    NULL);

    XFree(preedit_list);
    if (!xic) {
        printf("Failed to initialize xic\n");
    }

    // XSetICFocus(xic);

    // New: Load a font
    // font_info = XLoadQueryFont(display, "fixed");
    // if (font_info) {
        //XSetFont(display, gc, font_info->fid);
    // }

    // Main event loop
    XEvent event;
    while (1) {
        XNextEvent(display, &event);

        if (event.type == Expose) {
            // This is where you display the text
            XDrawString(display, window, gc, 50, 50, text_to_display, strlen(text_to_display));
        } else if (event.type == KeyPress) {
            char buf[32];
            KeySym keysym;
            Status status;
            int len;

            len = Xutf8LookupString(xic, &event.xkey, buf, sizeof(buf), &keysym, &status);
            if (len > 0) {
                // Here's where you would use the input, e.g., print to console
                printf("Received input: %.*s\n", len, buf);
            }
        }
    }

    // Cleanup
    XDestroyIC(xic);
    XCloseIM(xim);
    XFreeGC(display, gc);
    XCloseDisplay(display);

    return 0;
}
