#include <X11/Xlib.h>
#include <X11/Xatom.h>
#include <stdio.h>
#include <unistd.h>
static Window make_client(Display *d, Window root, const char *name, Atom desktop, Atom state, Atom above, Atom below, Atom hidden, Atom wm_state, int kind) {
 Window w=XCreateSimpleWindow(d,root,80+kind*40,90+kind*35,420,260,0,0,0x20242b); XStoreName(d,w,name);
 unsigned long ws=0; XChangeProperty(d,w,desktop,XA_CARDINAL,32,PropModeReplace,(unsigned char*)&ws,1);
 Atom states[1]; int n=0; if(kind==0)states[n++]=above; if(kind==1)states[n++]=below;
 if(kind==2){states[n++]=hidden; unsigned long iconic[2]={3,0}; XChangeProperty(d,w,wm_state,wm_state,32,PropModeReplace,(unsigned char*)iconic,2);}
 if(n)XChangeProperty(d,w,state,XA_ATOM,32,PropModeReplace,(unsigned char*)states,n);
 if(kind!=2)XMapWindow(d,w); return w;
}
int main(void){Display*d=XOpenDisplay(NULL);if(!d)return 2;Window root=DefaultRootWindow(d);
 Atom desktop=XInternAtom(d,"_NET_WM_DESKTOP",False),state=XInternAtom(d,"_NET_WM_STATE",False),above=XInternAtom(d,"_NET_WM_STATE_ABOVE",False),below=XInternAtom(d,"_NET_WM_STATE_BELOW",False),hidden=XInternAtom(d,"_NET_WM_STATE_HIDDEN",False),wm_state=XInternAtom(d,"WM_STATE",False);
 Window a=make_client(d,root,"MRX startup ABOVE",desktop,state,above,below,hidden,wm_state,0),b=make_client(d,root,"MRX startup BELOW",desktop,state,above,below,hidden,wm_state,1),h=make_client(d,root,"MRX startup ICONIC",desktop,state,above,below,hidden,wm_state,2);
 XSync(d,False);printf("%lu %lu %lu\n",a,b,h);fflush(stdout);for(;;)sleep(1);
}