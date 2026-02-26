#![allow(deprecated, unexpected_cfgs)]

use crate::AppCommand;
use cocoa::{
    appkit::{
        NSApplication, NSApplicationActivationPolicyRegular, NSButton, NSEvent, NSScreen,
        NSSquareStatusItemLength, NSStatusBar, NSStatusItem, NSWindow,
    },
    base::{id, nil, NO, YES},
    foundation::{NSArray, NSInteger, NSPoint, NSRect, NSSize, NSString},
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
use simple_term::terminal_settings::MonitorWindowPlacement;
use std::collections::HashMap;
use std::ffi::c_void;
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

#[derive(Clone)]
pub(crate) struct PanelPlacement {
    pub bounds: Bounds<Pixels>,
    pub top_left_x: f64,
    pub top_left_y: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MoveWindowResult {
    ActivationHandledByApp,
    ActivationDeferredToNative,
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
    monitor_window_positions: &HashMap<String, MonitorWindowPlacement>,
) -> PanelPlacement {
    unsafe {
        let screen = screen_for_mouse();
        let screen_frame = NSScreen::frame(screen);
        let visible_frame = NSScreen::visibleFrame(screen);
        let monitor_key = monitor_key_for_screen(screen_frame);

        let saved = monitor_window_positions.get(&monitor_key);
        let width = saved
            .and_then(|placement| placement.width)
            .unwrap_or(desired_width)
            .max(320.0)
            .min(visible_frame.size.width as f32);
        let height = saved
            .and_then(|placement| placement.height)
            .unwrap_or(desired_height)
            .max(180.0)
            .min(visible_frame.size.height as f32);

        let max_y = screen_frame.origin.y + screen_frame.size.height;
        let visible_max_y = visible_frame.origin.y + visible_frame.size.height;
        let min_local_x = (visible_frame.origin.x - screen_frame.origin.x) as f32;
        let max_local_x = ((visible_frame.origin.x + visible_frame.size.width)
            - screen_frame.origin.x
            - width as f64) as f32;
        let menubar_reserved = (max_y - visible_max_y).max(0.0) as f32;
        let min_local_y = menubar_reserved;
        let max_local_y = (max_y - visible_frame.origin.y - height as f64) as f32;

        let fallback_local_x =
            ((screen_frame.size.width as f32 - width) / 2.0).clamp(min_local_x, max_local_x);
        let fallback_local_y =
            (menubar_reserved + top_inset.max(0.0)).clamp(min_local_y, max_local_y);
        let (local_x, local_y) = saved
            .map(|saved| {
                (
                    saved.x.clamp(min_local_x, max_local_x),
                    saved.y.clamp(min_local_y, max_local_y),
                )
            })
            .unwrap_or((fallback_local_x, fallback_local_y));

        let top_left_x = screen_frame.origin.x + local_x as f64;
        let top_left_y = max_y - local_y as f64;

        PanelPlacement {
            bounds: Bounds::new(point(px(local_x), px(local_y)), size(px(width), px(height))),
            top_left_x,
            top_left_y,
        }
    }
}

pub(crate) fn capture_window_monitor_position(
    window: &mut Window,
) -> Option<(String, MonitorWindowPlacement)> {
    let Ok(window_handle) = window.window_handle() else {
        return None;
    };

    let RawWindowHandle::AppKit(handle) = window_handle.as_raw() else {
        return None;
    };

    unsafe {
        let ns_view = handle.ns_view.as_ptr() as id;
        let ns_window: id = msg_send![ns_view, window];
        if ns_window == nil {
            return None;
        }

        let mut screen: id = msg_send![ns_window, screen];
        if screen == nil {
            screen = NSScreen::mainScreen(nil);
            if screen == nil {
                screen = screen_for_mouse();
            }
        }

        if screen == nil {
            return None;
        }

        let screen_frame = NSScreen::frame(screen);
        let monitor_key = monitor_key_for_screen(screen_frame);

        let frame = NSWindow::frame(ns_window);
        let max_y = screen_frame.origin.y + screen_frame.size.height;
        let top_left_y = frame.origin.y + frame.size.height;
        let local_x = (frame.origin.x - screen_frame.origin.x) as f32;
        let local_y = (max_y - top_left_y) as f32;

        Some((
            monitor_key,
            MonitorWindowPlacement {
                x: local_x,
                y: local_y,
                width: Some(frame.size.width as f32),
                height: Some(frame.size.height as f32),
            },
        ))
    }
}

pub(crate) fn window_needs_frame_update(
    window: &mut Window,
    placement: &PanelPlacement,
    tolerance: f32,
) -> bool {
    let Ok(window_handle) = window.window_handle() else {
        return true;
    };

    let RawWindowHandle::AppKit(handle) = window_handle.as_raw() else {
        return true;
    };

    unsafe {
        let ns_view = handle.ns_view.as_ptr() as id;
        let ns_window: id = msg_send![ns_view, window];
        if ns_window == nil {
            return true;
        }

        let frame = NSWindow::frame(ns_window);
        let target_width = f32::from(placement.bounds.size.width) as f64;
        let target_height = f32::from(placement.bounds.size.height) as f64;
        let target_x = placement.top_left_x;
        let target_y = placement.top_left_y - target_height;
        let tolerance = tolerance.max(0.0) as f64;

        (frame.origin.x - target_x).abs() > tolerance
            || (frame.origin.y - target_y).abs() > tolerance
            || (frame.size.width - target_width).abs() > tolerance
            || (frame.size.height - target_height).abs() > tolerance
    }
}

pub(crate) fn move_window_to(
    window: &mut Window,
    placement: &PanelPlacement,
    activate_if_hidden: bool,
) -> MoveWindowResult {
    let Ok(window_handle) = window.window_handle() else {
        return MoveWindowResult::ActivationHandledByApp;
    };

    let RawWindowHandle::AppKit(handle) = window_handle.as_raw() else {
        return MoveWindowResult::ActivationHandledByApp;
    };

    unsafe {
        let ns_view = handle.ns_view.as_ptr() as id;
        let ns_window: id = msg_send![ns_view, window];
        if ns_window == nil {
            return MoveWindowResult::ActivationHandledByApp;
        }
        let is_visible: bool = msg_send![ns_window, isVisible];
        let activate_after_move = activate_if_hidden && !is_visible;

        schedule_window_frame_on_main_queue(
            ns_window,
            placement.top_left_x,
            placement.top_left_y,
            f32::from(placement.bounds.size.width) as f64,
            f32::from(placement.bounds.size.height) as f64,
            activate_after_move,
        );

        if activate_after_move {
            MoveWindowResult::ActivationDeferredToNative
        } else {
            MoveWindowResult::ActivationHandledByApp
        }
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

fn monitor_key_for_screen(frame: NSRect) -> String {
    format!(
        "{:.3}:{:.3}:{:.3}:{:.3}",
        frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
    )
}

extern "C" fn on_status_item_pressed(_: &Object, _: Sel, _: id) {
    if let Some(sender) = COMMAND_SENDER.get() {
        let _ = sender.try_send(AppCommand::ToggleTerminal);
    }
}

#[repr(C)]
struct DispatchQueueOpaque {
    _private: [u8; 0],
}

type DispatchQueue = *mut DispatchQueueOpaque;

#[repr(C)]
struct DeferredWindowMove {
    ns_window: id,
    top_left_x: f64,
    top_left_y: f64,
    width: f64,
    height: f64,
    activate_after_move: bool,
}

#[link(name = "System", kind = "dylib")]
unsafe extern "C" {
    #[link_name = "_dispatch_main_q"]
    static DISPATCH_MAIN_Q: DispatchQueueOpaque;
    fn dispatch_async_f(
        queue: DispatchQueue,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

unsafe fn schedule_window_frame_on_main_queue(
    ns_window: id,
    top_left_x: f64,
    top_left_y: f64,
    width: f64,
    height: f64,
    activate_after_move: bool,
) {
    let retained_window: id = msg_send![ns_window, retain];
    let context = Box::new(DeferredWindowMove {
        ns_window: retained_window,
        top_left_x,
        top_left_y,
        width,
        height,
        activate_after_move,
    });

    dispatch_async_f(
        (&raw const DISPATCH_MAIN_Q as *const DispatchQueueOpaque) as DispatchQueue,
        Box::into_raw(context).cast::<c_void>(),
        apply_deferred_window_move,
    );
}

unsafe extern "C" fn apply_deferred_window_move(context: *mut c_void) {
    let context = Box::from_raw(context.cast::<DeferredWindowMove>());
    let frame = window_frame_from_top_left(
        context.top_left_x,
        context.top_left_y,
        context.width,
        context.height,
    );
    let display = if context.activate_after_move { NO } else { YES };
    let _: () = msg_send![context.ns_window, setFrame: frame display: display];
    if context.activate_after_move {
        let app = NSApplication::sharedApplication(nil);
        let _: () = msg_send![app, unhide: nil];
        let _: () = msg_send![context.ns_window, makeKeyAndOrderFront: nil];
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    }
    let _: () = msg_send![context.ns_window, release];
}

fn window_frame_from_top_left(top_left_x: f64, top_left_y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(top_left_x, top_left_y - height),
        NSSize::new(width, height),
    )
}
