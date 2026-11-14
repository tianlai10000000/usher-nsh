//! AppKit controls for the Apple appearance settings surface.
//!
//! Settings text fields reuse NSH's native NSTextField integration, including
//! keyboard input and the shared AppKit field editor.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use nsh_core::JsonValue;
use nsh_platform_macos::{
    ComponentContext, Components, NativeComponent,
    objc2::{
        DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send,
        rc::Retained, sel,
    },
    objc2_app_kit::{
        NSAccessibility, NSAutoresizingMaskOptions, NSBezelStyle, NSButton, NSButtonType,
        NSControl, NSControlStateValueOff, NSControlStateValueOn, NSPopUpButton,
        NSSegmentSwitchTracking, NSSegmentedControl, NSTextAlignment, NSView,
    },
    objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString},
};

pub(crate) fn register(components: &Components) {
    let mtm = MainThreadMarker::new().expect("AppKit components must register on the main thread");
    components.register(
        "usher.settings.segments",
        Rc::new(move |context| {
            let segments = NSSegmentedControl::new(mtm);
            segments.setTrackingMode(NSSegmentSwitchTracking::SelectOne);
            Box::new(SettingsControl::new(
                Control::Segments(segments),
                context,
                mtm,
            ))
        }),
    );
    components.register(
        "usher.settings.popup",
        Rc::new(move |context| {
            let popup = NSPopUpButton::initWithFrame_pullsDown(mtm.alloc(), NSRect::ZERO, false);
            popup.setAutoenablesItems(false);
            Box::new(SettingsControl::new(Control::Popup(popup), context, mtm))
        }),
    );
    components.register(
        "usher.settings.sidebar",
        Rc::new(move |context| Box::new(SidebarControl::new(context, mtm))),
    );
    components.register(
        "usher.settings.button",
        Rc::new(move |context| {
            let button = NSButton::new(mtm);
            button.setButtonType(NSButtonType::MomentaryPushIn);
            button.setBezelStyle(NSBezelStyle::Push);
            Box::new(SettingsControl::new(Control::Button(button), context, mtm))
        }),
    );
}

#[derive(PartialEq, Eq)]
struct Choice {
    id: String,
    label: String,
}

impl Choice {
    fn from_value(value: &JsonValue) -> Option<Self> {
        Some(Self {
            id: value.get("id")?.as_str()?.to_owned(),
            label: value.get("label")?.as_str()?.to_owned(),
        })
    }
}

struct ActionState {
    context: ComponentContext,
    choices: RefCell<Vec<Choice>>,
    interaction_blocked: Cell<bool>,
    enabled: Cell<bool>,
    selected_index: Cell<Option<usize>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ActionState]
    #[name = "UsherSettingsControlTarget"]
    struct ActionTarget;

    impl ActionTarget {
        #[unsafe(method(selectSegment:))]
        fn select_segment(&self, sender: &NSSegmentedControl) {
            if self.blocked() {
                if let Some(index) = self.ivars().selected_index.get() {
                    sender.setSelectedSegment(index as isize);
                } else {
                    sender.setSelectedSegment(-1);
                }
                return;
            }
            self.emit_selection(sender.selectedSegment());
        }

        #[unsafe(method(selectPopupItem:))]
        fn select_popup_item(&self, sender: &NSPopUpButton) {
            if self.blocked() {
                match self.ivars().selected_index.get() {
                    Some(index) => {
                        sender.selectItemAtIndex(index as isize);
                        sender.synchronizeTitleAndSelectedItem();
                    }
                    None => sender.selectItemAtIndex(-1),
                }
                return;
            }
            self.emit_selection(sender.indexOfSelectedItem());
        }

        #[unsafe(method(selectSidebarItem:))]
        fn select_sidebar_item(&self, sender: &NSButton) {
            self.emit_selection(sender.tag());
        }

        #[unsafe(method(activate:))]
        fn activate(&self, _sender: &NSButton) {
            if self.blocked() { return; }
            self.ivars().context.emit("activate", JsonValue::Object(Default::default()));
        }
    }
);

impl ActionTarget {
    fn blocked(&self) -> bool {
        self.ivars().interaction_blocked.get() || !self.ivars().enabled.get()
    }

    fn new(context: ComponentContext, mtm: MainThreadMarker) -> Retained<Self> {
        let target = Self::alloc(mtm).set_ivars(ActionState {
            context,
            choices: RefCell::default(),
            interaction_blocked: Cell::new(false),
            enabled: Cell::new(true),
            selected_index: Cell::new(None),
        });
        // SAFETY: NSObject's initializer leaves the ivars installed by objc2 intact.
        unsafe { msg_send![super(target), init] }
    }

    fn emit_selection(&self, index: isize) {
        let Some(id) = usize::try_from(index).ok().and_then(|index| {
            self.ivars()
                .choices
                .borrow()
                .get(index)
                .map(|choice| choice.id.clone())
        }) else {
            return;
        };
        let data = JsonValue::Object([("id".into(), JsonValue::from(id))].into_iter().collect());
        self.ivars().context.emit("change", data);
    }
}

enum Control {
    Segments(Retained<NSSegmentedControl>),
    Popup(Retained<NSPopUpButton>),
    Button(Retained<NSButton>),
}

impl Control {
    fn as_control(&self) -> &NSControl {
        match self {
            Self::Segments(control) => control,
            Self::Popup(control) => control,
            Self::Button(control) => control,
        }
    }

    fn set_choices(&self, choices: &[Choice]) {
        match self {
            Self::Segments(segments) => {
                segments.setSegmentCount(choices.len() as isize);
                for (index, choice) in choices.iter().enumerate() {
                    segments
                        .setLabel_forSegment(&NSString::from_str(&choice.label), index as isize);
                }
            }
            Self::Popup(popup) => {
                popup.removeAllItems();
                // AppKit replaces existing items with the same title when adding.
                // Create distinct items first so two themes can share a display name.
                for index in 0..choices.len() {
                    popup.addItemWithTitle(&NSString::from_str(&index.to_string()));
                }
                for (index, choice) in choices.iter().enumerate() {
                    if let Some(item) = popup.itemAtIndex(index as isize) {
                        item.setTitle(&NSString::from_str(&choice.label));
                    }
                }
            }
            Self::Button(_) => {}
        }
    }

    fn select(&self, index: Option<usize>) {
        match self {
            Self::Segments(segments) => {
                if let Some(index) = index {
                    segments.setSelectedSegment(index as isize);
                } else {
                    segments.setSelectedSegment(-1);
                }
            }
            Self::Popup(popup) => match index {
                Some(index) => {
                    popup.selectItemAtIndex(index as isize);
                    popup.synchronizeTitleAndSelectedItem();
                }
                // Title synchronization would implicitly select the first item when empty.
                None => popup.selectItemAtIndex(-1),
            },
            Self::Button(_) => {}
        }
    }
}

struct SettingsControl {
    control: Control,
    target: Retained<ActionTarget>,
    selected_id: Option<String>,
}

impl SettingsControl {
    fn new(control: Control, context: ComponentContext, mtm: MainThreadMarker) -> Self {
        let target = ActionTarget::new(context, mtm);
        let action = match &control {
            Control::Segments(_) => sel!(selectSegment:),
            Control::Popup(_) => sel!(selectPopupItem:),
            Control::Button(_) => sel!(activate:),
        };
        // SAFETY: Each selector accepts the matching control type. The wrapper
        // retains the target until Drop clears the non-owning AppKit reference.
        unsafe {
            control.as_control().setTarget(Some(&target));
            control.as_control().setAction(Some(action));
        }
        Self {
            control,
            target,
            selected_id: None,
        }
    }

    fn refresh_selection(&self) {
        let choices = self.target.ivars().choices.borrow();
        let index = self
            .selected_id
            .as_ref()
            .and_then(|id| choices.iter().position(|choice| &choice.id == id));
        self.target.ivars().selected_index.set(index);
        self.control.select(index);
    }

    fn report_size(&self) {
        let size = self.control.as_control().intrinsicContentSize();
        self.target
            .ivars()
            .context
            .set_size(size.width.max(1.0) as f32, size.height.max(24.0) as f32);
    }
}

impl NativeComponent for SettingsControl {
    fn view(&self) -> Retained<NSView> {
        Retained::into_super(self.control.as_control().retain())
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        match name {
            "items" => {
                let choices = value
                    .and_then(JsonValue::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Choice::from_value)
                    .collect::<Vec<_>>();
                if *self.target.ivars().choices.borrow() != choices {
                    self.control.set_choices(&choices);
                    self.target.ivars().choices.replace(choices);
                    self.refresh_selection();
                    self.report_size();
                }
            }
            "selectedId" => {
                self.selected_id = value.and_then(JsonValue::as_str).map(str::to_owned);
                self.refresh_selection();
            }
            "interactionBlocked" => {
                self.target
                    .ivars()
                    .interaction_blocked
                    .set(value.and_then(JsonValue::as_bool).unwrap_or(false));
            }
            "enabled" => {
                let enabled = value.and_then(JsonValue::as_bool).unwrap_or(true);
                self.target.ivars().enabled.set(enabled);
                self.control.as_control().setEnabled(enabled);
                if enabled {
                    self.refresh_selection();
                }
            }
            "label" => {
                let label = value.and_then(JsonValue::as_str).map(NSString::from_str);
                if let Control::Button(button) = &self.control {
                    button.setAccessibilityLabel(label.as_deref());
                    button.setTitle(&label.unwrap_or_default());
                    self.report_size();
                } else {
                    self.control
                        .as_control()
                        .setAccessibilityLabel(label.as_deref());
                }
            }
            _ => {}
        }
    }
}

impl Drop for SettingsControl {
    fn drop(&mut self) {
        // SAFETY: The control and its target are both still alive.
        unsafe {
            self.control.as_control().setAction(None);
            self.control.as_control().setTarget(None);
        }
    }
}

/// One row's height and the gap that follows it, in points. The TSX side sizes the
/// control as `items * SIDEBAR_ROW`, so a row never depends on a reported size.
const SIDEBAR_ROW: f64 = 32.0;
const SIDEBAR_BUTTON: f64 = 28.0;

/// A vertical list of native toolbar-style buttons: the settings section navigation
/// for the AppKit family. The selected row shows the pressed (on) state.
struct SidebarControl {
    container: Retained<NSView>,
    buttons: Vec<Retained<NSButton>>,
    target: Retained<ActionTarget>,
    selected_id: Option<String>,
    mtm: MainThreadMarker,
}

impl SidebarControl {
    fn new(context: ComponentContext, mtm: MainThreadMarker) -> Self {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(180.0, SIDEBAR_ROW));
        Self {
            container: NSView::initWithFrame(NSView::alloc(mtm), frame),
            buttons: Vec::new(),
            target: ActionTarget::new(context, mtm),
            selected_id: None,
            mtm,
        }
    }

    fn rebuild(&mut self, choices: &[Choice]) {
        for button in self.buttons.drain(..) {
            // SAFETY: the target outlives the button; clearing stops later callbacks.
            unsafe {
                button.setAction(None);
                button.setTarget(None);
            }
            button.removeFromSuperview();
        }
        let height = choices.len() as f64 * SIDEBAR_ROW;
        let width = self.container.frame().size.width;
        self.container
            .setFrameSize(NSSize::new(width, height.max(SIDEBAR_ROW)));
        for (index, choice) in choices.iter().enumerate() {
            let button = NSButton::new(self.mtm);
            button.setTitle(&NSString::from_str(&choice.label));
            button.setButtonType(NSButtonType::PushOnPushOff);
            button.setBezelStyle(NSBezelStyle::AccessoryBarAction);
            button.setAlignment(NSTextAlignment::Left);
            button.setTag(index as isize);
            // The view is not flipped: row 0 sits at the top.
            let y =
                height - (index as f64 + 1.0) * SIDEBAR_ROW + (SIDEBAR_ROW - SIDEBAR_BUTTON) / 2.0;
            button.setFrame(NSRect::new(
                NSPoint::new(0.0, y),
                NSSize::new(width, SIDEBAR_BUTTON),
            ));
            // Follow the width; keep the top margin fixed when the height changes.
            button.setAutoresizingMask(
                NSAutoresizingMaskOptions::ViewWidthSizable
                    | NSAutoresizingMaskOptions::ViewMinYMargin,
            );
            // SAFETY: the selector takes the sending button; the target is retained here.
            unsafe {
                button.setTarget(Some(&self.target));
                button.setAction(Some(sel!(selectSidebarItem:)));
            }
            self.container.addSubview(&button);
            self.buttons.push(button);
        }
    }

    fn refresh_selection(&self) {
        let choices = self.target.ivars().choices.borrow();
        let selected = self
            .selected_id
            .as_ref()
            .and_then(|id| choices.iter().position(|choice| &choice.id == id));
        for (index, button) in self.buttons.iter().enumerate() {
            button.setState(if Some(index) == selected {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    }
}

impl NativeComponent for SidebarControl {
    fn view(&self) -> Retained<NSView> {
        self.container.clone()
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        match name {
            "items" => {
                let choices = value
                    .and_then(JsonValue::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Choice::from_value)
                    .collect::<Vec<_>>();
                if *self.target.ivars().choices.borrow() != choices {
                    self.rebuild(&choices);
                    self.target.ivars().choices.replace(choices);
                    self.refresh_selection();
                }
            }
            "selectedId" => {
                self.selected_id = value.and_then(JsonValue::as_str).map(str::to_owned);
                self.refresh_selection();
            }
            "enabled" => {
                let enabled = value.and_then(JsonValue::as_bool).unwrap_or(true);
                for button in &self.buttons {
                    button.setEnabled(enabled);
                }
            }
            "label" => {
                let label = value.and_then(JsonValue::as_str).map(NSString::from_str);
                self.container.setAccessibilityLabel(label.as_deref());
            }
            _ => {}
        }
    }
}

impl Drop for SidebarControl {
    fn drop(&mut self) {
        for button in &self.buttons {
            // SAFETY: the buttons and their target are both still alive.
            unsafe {
                button.setAction(None);
                button.setTarget(None);
            }
        }
    }
}
