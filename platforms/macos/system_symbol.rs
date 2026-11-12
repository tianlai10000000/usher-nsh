//! SF Symbols drawn by AppKit, so sidebar and toolbar icons match the system.

use std::rc::Rc;

use nsh_core::JsonValue;
use nsh_platform_macos::objc2::MainThreadMarker;
use nsh_platform_macos::objc2::rc::Retained;
use nsh_platform_macos::objc2_app_kit::{NSColor, NSImage, NSImageScaling, NSImageView, NSView};
use nsh_platform_macos::objc2_foundation::NSString;
use nsh_platform_macos::{ComponentContext, Components, NativeComponent};

pub(crate) fn register(components: &Components) {
    let mtm = MainThreadMarker::new().expect("AppKit components must register on the main thread");
    components.register(
        "UsherSystemSymbol",
        Rc::new(move |context| {
            let view = NSImageView::new(mtm);
            view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
            Box::new(SystemSymbol {
                view,
                context,
                size: 16.0,
            })
        }),
    );
}

struct SystemSymbol {
    view: Retained<NSImageView>,
    context: ComponentContext,
    size: f64,
}

/// Parses `#rrggbb` or `#rrggbbaa`.
fn parse_color(text: &str) -> Option<Retained<NSColor>> {
    let hex = text.strip_prefix('#')?;
    if !(hex.len() == 6 || hex.len() == 8) {
        return None;
    }
    let channel = |index: usize| {
        u8::from_str_radix(hex.get(index * 2..index * 2 + 2)?, 16)
            .ok()
            .map(|value| f64::from(value) / 255.0)
    };
    let alpha = if hex.len() == 8 { channel(3)? } else { 1.0 };
    Some(NSColor::colorWithSRGBRed_green_blue_alpha(
        channel(0)?,
        channel(1)?,
        channel(2)?,
        alpha,
    ))
}

impl NativeComponent for SystemSymbol {
    fn view(&self) -> Retained<NSView> {
        Retained::into_super(Retained::into_super(self.view.clone()))
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        match name {
            "name" => {
                let image = value.and_then(JsonValue::as_str).and_then(|symbol| {
                    NSImage::imageWithSystemSymbolName_accessibilityDescription(
                        &NSString::from_str(symbol),
                        None,
                    )
                });
                self.view.setImage(image.as_deref());
            }
            "size" => {
                if let Some(size) = value.and_then(JsonValue::as_f64) {
                    self.size = size.max(1.0);
                    self.context.set_size(self.size as f32, self.size as f32);
                }
            }
            "color" => {
                let color = value.and_then(JsonValue::as_str).and_then(parse_color);
                self.view.setContentTintColor(color.as_deref());
            }
            _ => {}
        }
    }
}
