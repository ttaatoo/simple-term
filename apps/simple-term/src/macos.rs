#![allow(deprecated, unexpected_cfgs)]

use crate::AppCommand;
use cocoa::{
    appkit::{
        NSApplication, NSApplicationActivationPolicyRegular, NSButton, NSEvent, NSScreen,
        NSSquareStatusItemLength, NSStatusBar, NSStatusItem, NSWindow,
    },
    base::{id, nil},
    foundation::{NSArray, NSInteger, NSPoint, NSRect, NSString},
};
use gpui::{point, px, size, Bounds, Pixels, Window};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::StrongPtr,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ptr;
use std::sync::{Once, OnceLock};

static COMMAND_SENDER: OnceLock<smol::channel::Sender<AppCommand>> = OnceLock::new();
static STATUS_ITEM_CLASS_INIT: Once = Once::new();
static mut STATUS_ITEM_TARGET_CLASS: *const Class = ptr::null();
const NS_WINDOW_LEVEL_NORMAL: NSInteger = 0;
const NS_WINDOW_LEVEL_FLOATING: NSInteger = 3;

pub(crate) struct StatusItemHandle {
    status_item: StrongPtr,
    target: StrongPtr,
}

impl Drop for StatusItemHandle {
    fn drop(&mut self) {
        unsafe {
            let status_bar = NSStatusBar::systemStatusBar(nil);
            status_bar.removeStatusItem_(*self.status_item);
        }

        let _ = &self.target;
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PanelPlacement {
    pub bounds: Bounds<Pixels>,
    pub top_left_x: f64,
    pub top_left_y: f64,
}

pub(crate) fn set_command_sender(sender: smol::channel::Sender<AppCommand>) {
    let _ = COMMAND_SENDER.set(sender);
}

pub(crate) fn apply_app_activation_policy() {
    unsafe {
        let app = NSApplication::sharedApplication(nil);
        let _ = app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
    }
}

pub(crate) fn install_status_item(enabled: bool) -> Option<StatusItemHandle> {
    if !enabled {
        return None;
    }

    unsafe {
        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item =
            StrongPtr::retain(status_bar.statusItemWithLength_(NSSquareStatusItemLength));

        let button = status_item.button();
        if button == nil {
            log::warn!("failed to create menubar button");
            return None;
        }

        let title = NSString::alloc(nil).init_str("⌥");
        NSButton::setTitle_(button, title);

        let target: id = msg_send![status_item_target_class(), new];
        let _: () = msg_send![button, setTarget: target];
        let _: () = msg_send![button, setAction: sel!(statusItemPressed:)];

        Some(StatusItemHandle {
            status_item,
            target: StrongPtr::new(target),
        })
    }
}

pub(crate) fn resolve_panel_placement(
    desired_width: f32,
    desired_height: f32,
    top_inset: f32,
) -> PanelPlacement {
    unsafe {
        let screen = screen_for_mouse();
        let screen_frame = NSScreen::frame(screen);
        let visible_frame = NSScreen::visibleFrame(screen);

        let width = desired_width.max(320.0).min(screen_frame.size.width as f32);
        let height = desired_height
            .max(180.0)
            .min(visible_frame.size.height as f32);

        let local_x = ((screen_frame.size.width as f32 - width) / 2.0).max(0.0);
        let max_y = screen_frame.origin.y + screen_frame.size.height;
        let visible_max_y = visible_frame.origin.y + visible_frame.size.height;
        let menubar_reserved = (max_y - visible_max_y).max(0.0) as f32;
        let local_y = menubar_reserved + top_inset.max(0.0);

        let top_left_x = screen_frame.origin.x + local_x as f64;
        let top_left_y = max_y - local_y as f64;

        PanelPlacement {
            bounds: Bounds::new(point(px(local_x), px(local_y)), size(px(width), px(height))),
            top_left_x,
            top_left_y,
        }
    }
}

pub(crate) fn move_window_to(window: &mut Window, placement: &PanelPlacement) {
    let Ok(window_handle) = window.window_handle() else {
        return;
    };

    let RawWindowHandle::AppKit(handle) = window_handle.as_raw() else {
        return;
    };

    unsafe {
        let ns_view = handle.ns_view.as_ptr() as id;
        let ns_window: id = msg_send![ns_view, window];
        if ns_window == nil {
            return;
        }

        let top_left = NSPoint::new(placement.top_left_x, placement.top_left_y);
        NSWindow::setFrameTopLeftPoint_(ns_window, top_left);
    }
}

pub(crate) fn set_window_pinned(window: &mut Window, pinned: bool) {
    let Ok(window_handle) = window.window_handle() else {
        return;
    };

    let RawWindowHandle::AppKit(handle) = window_handle.as_raw() else {
        return;
    };

    unsafe {
        let ns_view = handle.ns_view.as_ptr() as id;
        let ns_window: id = msg_send![ns_view, window];
        if ns_window == nil {
            return;
        }

        let level = if pinned {
            NS_WINDOW_LEVEL_FLOATING
        } else {
            NS_WINDOW_LEVEL_NORMAL
        };
        NSWindow::setLevel_(ns_window, level);
    }
}

unsafe fn status_item_target_class() -> *const Class {
    STATUS_ITEM_CLASS_INIT.call_once(|| {
        let mut decl = ClassDecl::new("SimpleTermStatusItemTarget", class!(NSObject))
            .expect("failed to create status item target class");
        decl.add_method(
            sel!(statusItemPressed:),
            on_status_item_pressed as extern "C" fn(&Object, Sel, id),
        );
        STATUS_ITEM_TARGET_CLASS = decl.register();
    });

    STATUS_ITEM_TARGET_CLASS
}

unsafe fn screen_for_mouse() -> id {
    let mouse = NSEvent::mouseLocation(nil);
    let screens = NSScreen::screens(nil);
    let count = NSArray::count(screens);

    for i in 0..count {
        let screen = NSArray::objectAtIndex(screens, i);
        let frame = NSScreen::frame(screen);
        if point_in_rect(mouse, frame) {
            return screen;
        }
    }

    let main_screen = NSScreen::mainScreen(nil);
    if main_screen != nil {
        main_screen
    } else {
        NSArray::objectAtIndex(screens, 0)
    }
}

fn point_in_rect(point: NSPoint, rect: NSRect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.height
}

extern "C" fn on_status_item_pressed(_: &Object, _: Sel, _: id) {
    if let Some(sender) = COMMAND_SENDER.get() {
        let _ = sender.try_send(AppCommand::ToggleTerminal);
    }
}
