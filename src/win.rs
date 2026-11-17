#![allow(non_snake_case, non_camel_case_types, dead_code)]
use std::ffi::c_void;
pub type Handle = *mut c_void;
pub type Hwnd = Handle;
pub type Hdc = Handle;
pub const NULL: Handle = std::ptr::null_mut();
pub fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct FileTime { pub low: u32, pub high: u32 }
#[repr(C)] #[derive(Default, Clone, Copy, Debug)]
pub struct SystemTime { pub year:u16, pub month:u16, pub weekday:u16, pub day:u16, pub hour:u16, pub minute:u16, pub second:u16, pub millis:u16 }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct Rect { pub left:i32, pub top:i32, pub right:i32, pub bottom:i32 }
#[repr(C)] #[derive(Default, Clone, Copy)]
pub struct Point { pub x:i32, pub y:i32 }
#[repr(C)] pub struct WndClass {
    pub size:u32, pub style:u32, pub proc:Option<unsafe extern "system" fn(Hwnd,u32,usize,isize)->isize>,
    pub cls_extra:i32, pub wnd_extra:i32, pub instance:Handle, pub icon:Handle, pub cursor:Handle,
    pub background:Handle, pub menu:*const u16, pub name:*const u16, pub small_icon:Handle,
}
#[repr(C)] #[derive(Default)]
pub struct Message { pub hwnd:Hwnd, pub message:u32, pub wparam:usize, pub lparam:isize, pub time:u32, pub point:Point, pub private:u32 }
#[repr(C)]
pub struct Paint { pub dc:Hdc, pub erase:i32, pub rect:Rect, pub restore:i32, pub update:i32, pub reserved:[u8;32] }
#[repr(C)] pub struct MinMax { pub reserved:Point, pub max_size:Point, pub max_position:Point, pub min_track:Point, pub max_track:Point }
#[repr(C)] pub struct ScrollInfo { pub size:u32, pub mask:u32, pub min:i32, pub max:i32, pub page:u32, pub pos:i32, pub track:i32 }

#[link(name="kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleW(name:*const u16)->Handle;
    pub fn GetLastError()->u32;
    pub fn GetSystemTimeAsFileTime(t:*mut FileTime);
    pub fn FileTimeToSystemTime(t:*const FileTime,s:*mut SystemTime)->i32;
    pub fn SystemTimeToFileTime(s:*const SystemTime,t:*mut FileTime)->i32;
    pub fn SystemTimeToTzSpecificLocalTime(tz:Handle,utc:*const SystemTime,local:*mut SystemTime)->i32;
    pub fn TzSpecificLocalTimeToSystemTime(tz:Handle,local:*const SystemTime,utc:*mut SystemTime)->i32;
}
#[link(name="wevtapi")]
unsafe extern "system" {
    pub fn EvtQuery(session:Handle,path:*const u16,query:*const u16,flags:u32)->Handle;
    pub fn EvtNext(query:Handle,size:u32,events:*mut Handle,timeout:u32,flags:u32,returned:*mut u32)->i32;
    pub fn EvtRender(context:Handle,event:Handle,flags:u32,size:u32,buffer:*mut c_void,used:*mut u32,count:*mut u32)->i32;
    pub fn EvtClose(handle:Handle)->i32;
}
#[link(name="user32")]
unsafe extern "system" {
    pub fn SetProcessDpiAwarenessContext(context:Handle)->i32;
    pub fn GetDpiForWindow(hwnd:Hwnd)->u32;
    pub fn RegisterClassExW(class:*const WndClass)->u16;
    pub fn CreateWindowExW(ex:u32,class:*const u16,title:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:Hwnd,menu:Handle,instance:Handle,param:Handle)->Hwnd;
    pub fn DefWindowProcW(hwnd:Hwnd,msg:u32,wp:usize,lp:isize)->isize;
    pub fn ShowWindow(hwnd:Hwnd,command:i32)->i32;
    pub fn UpdateWindow(hwnd:Hwnd)->i32;
    pub fn GetMessageW(msg:*mut Message,hwnd:Hwnd,min:u32,max:u32)->i32;
    pub fn TranslateMessage(msg:*const Message)->i32;
    pub fn DispatchMessageW(msg:*const Message)->isize;
    pub fn PostQuitMessage(code:i32);
    pub fn PostMessageW(hwnd:Hwnd,msg:u32,wp:usize,lp:isize)->i32;
    pub fn GetClientRect(hwnd:Hwnd,rect:*mut Rect)->i32;
    pub fn LoadCursorW(instance:Handle,name:*const u16)->Handle;
    pub fn LoadIconW(instance:Handle,name:*const u16)->Handle;
    pub fn SetCursor(cursor:Handle)->Handle;
    pub fn BeginPaint(hwnd:Hwnd,paint:*mut Paint)->Hdc;
    pub fn EndPaint(hwnd:Hwnd,paint:*const Paint)->i32;
    pub fn InvalidateRect(hwnd:Hwnd,rect:*const Rect,erase:i32)->i32;
    pub fn FillRect(dc:Hdc,rect:*const Rect,brush:Handle)->i32;
    pub fn DrawTextW(dc:Hdc,text:*const u16,count:i32,rect:*mut Rect,format:u32)->i32;
    pub fn MessageBoxW(hwnd:Hwnd,text:*const u16,title:*const u16,kind:u32)->i32;
    pub fn SetTimer(hwnd:Hwnd,id:usize,interval:u32,proc:Handle)->usize;
    pub fn KillTimer(hwnd:Hwnd,id:usize)->i32;
    pub fn SetWindowLongPtrW(hwnd:Hwnd,index:i32,value:isize)->isize;
    pub fn GetWindowLongPtrW(hwnd:Hwnd,index:i32)->isize;
    pub fn SetScrollInfo(hwnd:Hwnd,bar:i32,info:*const ScrollInfo,redraw:i32)->i32;
    pub fn GetScrollInfo(hwnd:Hwnd,bar:i32,info:*mut ScrollInfo)->i32;
    pub fn GetCursorPos(point:*mut Point)->i32;
    pub fn ScreenToClient(hwnd:Hwnd,point:*mut Point)->i32;
}
#[link(name="gdi32")]
unsafe extern "system" {
    pub fn CreateCompatibleDC(dc:Hdc)->Hdc;
    pub fn CreateCompatibleBitmap(dc:Hdc,w:i32,h:i32)->Handle;
    pub fn SelectObject(dc:Hdc,object:Handle)->Handle;
    pub fn DeleteObject(object:Handle)->i32;
    pub fn DeleteDC(dc:Hdc)->i32;
    pub fn BitBlt(dest:Hdc,x:i32,y:i32,w:i32,h:i32,src:Hdc,sx:i32,sy:i32,rop:u32)->i32;
    pub fn CreateSolidBrush(color:u32)->Handle;
    pub fn CreatePen(style:i32,width:i32,color:u32)->Handle;
    pub fn RoundRect(dc:Hdc,left:i32,top:i32,right:i32,bottom:i32,ew:i32,eh:i32)->i32;
    pub fn MoveToEx(dc:Hdc,x:i32,y:i32,old:*mut Point)->i32;
    pub fn LineTo(dc:Hdc,x:i32,y:i32)->i32;
    pub fn CreateFontW(height:i32,width:i32,esc:i32,orientation:i32,weight:i32,italic:u32,underline:u32,strike:u32,charset:u32,out:u32,clip:u32,quality:u32,pitch:u32,face:*const u16)->Handle;
    pub fn SetTextColor(dc:Hdc,color:u32)->u32;
    pub fn SetBkMode(dc:Hdc,mode:i32)->i32;
    pub fn SetMapMode(dc:Hdc,mode:i32)->i32;
    pub fn SetWindowExtEx(dc:Hdc,x:i32,y:i32,old:*mut Point)->i32;
    pub fn SetViewportExtEx(dc:Hdc,x:i32,y:i32,old:*mut Point)->i32;
}
#[link(name="dwmapi")]
unsafe extern "system" { pub fn DwmSetWindowAttribute(hwnd:Hwnd,attribute:u32,value:*const c_void,size:u32)->i32; }
