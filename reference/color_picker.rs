// Color picker handler written against src (CanvasEvent in, Command out).
//
// state: hsl only. hex / swatch / slider are different views of it.
// - slider (Input event) edits one channel of hsl by its position, so a gray keeps its hue.
// - hex (Input event) replaces hsl. Anything but "#RRGGBB" is ignored.
// - swatch (Click event) replaces the lightness with the swatch's. The swatch at the current
//   lightness is the checked radio; there is no other state for it.
//
// Temporary wiring for verification (lib.rs), then remove it again:
//
//   #[cfg(test)]
//   #[path = "../reference/color_picker.rs"]
//   mod color_picker;
//
//   cargo test --no-default-features --lib color_picker
//
// DOM: reference/color_picker.html (ids are the dom::Id path, same as distribution/index.html)

use alloc::{format, string::String, vec, vec::Vec};
use core::{
    option::Option::{None, Some},
    primitive::{f64, u8, u32},
};

use crate::js_client::{
    Attribute::Checked,
    CanvasEvent, Command, EventType,
    StyleProperty::{Background, Color},
    StyleValue,
    dom::{Id, Tag},
};

const HSL_MAX: [f64; 3] = [360.0, 100.0, 100.0];
const HSL_SUFFIX: [&str; 3] = ["°", "%", "%"];
const SWATCH_LIGHTNESS: [f64; 7] = [10.0, 20.0, 35.0, 50.0, 65.0, 80.0, 90.0];

pub struct ColorPicker {
    hsl: [f64; 3],
}

impl ColorPicker {
    pub fn new() -> Self {
        Self { hsl: from_rgb([99, 102, 241]) }
    }

    pub fn process_canvas(&mut self, event: &CanvasEvent) -> Vec<Command> {
        match event.event_type {
            EventType::Input => {
                if let Some(channel) = (0..3).find(|&n| event.id == slider(n, Tag::Input)) {
                    match event.value.parse::<f64>() {
                        Ok(value) if value.is_finite() => {
                            self.hsl[channel] = value.clamp(0.0, HSL_MAX[channel]);
                        }
                        _ => return vec![],
                    }
                } else if event.id == hex_input() {
                    let digits = event.value.strip_prefix('#').unwrap_or("");
                    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return vec![];
                    }
                    let rgb = u32::from_str_radix(digits, 16).unwrap_or(0).to_be_bytes();
                    self.hsl = from_rgb([rgb[1], rgb[2], rgb[3]]);
                } else {
                    return vec![];
                }
            }
            EventType::Click => match (0..7).find(|&n| event.id == swatch(n, Tag::Input)) {
                Some(n) => self.hsl[2] = SWATCH_LIGHTNESS[n],
                None => return vec![],
            },
            _ => return vec![],
        }
        self.draw()
    }

    pub fn draw(&self) -> Vec<Command> {
        let [hue, saturation, lightness] = self.hsl;
        let rgb = to_rgb(self.hsl);
        let text = |id, value: String| Command::SetText { id, value };
        let style = |id, property, value: String| Command::SetStyle {
            id,
            property,
            value: StyleValue::Text(value),
        };

        let mut commands = vec![
            style(preview(), Background, hex(rgb)),
            Command::SetValue { id: hex_input(), value: hex(rgb) },
            style(hex_input(), Color, String::from(ink(rgb))),
            style(
                slider(1, Tag::Input),
                Background,
                format!("linear-gradient(to right, #808080, {})", hex(to_rgb([hue, 100.0, 50.0]))),
            ),
        ];
        for n in 0..3 {
            let value = format!("{:.0}", self.hsl[n]);
            commands.push(Command::SetValue { id: slider(n, Tag::Input), value: value.clone() });
            commands.push(text(slider(n, Tag::Output), format!("{value}{}", HSL_SUFFIX[n])));
        }
        for (n, l) in SWATCH_LIGHTNESS.into_iter().enumerate() {
            let rgb = to_rgb([hue, saturation, l]);
            let input = swatch(n, Tag::Input);
            commands.push(style(input.clone(), Background, hex(rgb)));
            commands.push(style(input.clone(), Color, String::from(ink(rgb))));
            commands.push(text(swatch(n, Tag::Span), hex(rgb)));
            commands.push(if libm::round(lightness) == l {
                Command::SetAttribute { id: input, attribute: Checked, value: String::new() }
            } else {
                Command::RemoveAttribute { id: input, attribute: Checked }
            });
        }
        commands
    }
}

// [h, s%, l%] -> [r, g, b]
fn to_rgb([h, s, l]: [f64; 3]) -> [u8; 3] {
    let s = s / 100.0;
    let l = l / 100.0;
    let a = s * l.min(1.0 - l);
    [0.0, 8.0, 4.0].map(|n| {
        let k = (n + h / 30.0) % 12.0;
        let v = l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        libm::round(v * 255.0) as u8
    })
}

// [r, g, b] -> [h, s%, l%]
fn from_rgb(rgb: [u8; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(|c| c as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let l = (max + min) / 2.0;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - libm::fabs(2.0 * l - 1.0)) };
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    [if h < 0.0 { h + 360.0 } else { h }, s * 100.0, l * 100.0]
}

fn hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn ink([r, g, b]: [u8; 3]) -> &'static str {
    let luminance = 0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64;
    if luminance > 127.5 { "#000000" } else { "#FFFFFF" }
}

fn preview() -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, Some(1)), (Tag::Div, None)])
}

fn hex_input() -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, Some(1)), (Tag::Div, None), (Tag::Input, None)])
}

/// n: 0 hue, 1 saturation, 2 lightness. tag: Input (range) or Output (text).
fn slider(n: usize, tag: Tag) -> Id {
    Id::new(&[(Tag::Main, None), (Tag::Section, Some(1)), (Tag::Label, Some(n as u32 + 1)), (tag, None)])
}

/// tag: Input (radio) or Span (hex text).
fn swatch(n: usize, tag: Tag) -> Id {
    Id::new(&[
        (Tag::Main, None),
        (Tag::Section, Some(1)),
        (Tag::Fieldset, None),
        (Tag::Label, Some(n as u32 + 1)),
        (tag, None),
    ])
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec::Vec};
    use core::unreachable;
    use std::fs;

    use super::*;
    use crate::{
        event::{EVENT_CANVAS, Event, decode_event},
        js_client::{KeyName, encode_command, put_f32, put_str, put_u32},
    };

    fn event(event_type: EventType, id: Id, value: &str) -> CanvasEvent {
        CanvasEvent {
            event_type,
            id,
            key: KeyName::Other,
            flags: 0,
            value: value.to_string(),
            x: 0.0,
            y: 0.0,
            local_x: 0.0,
            local_y: 0.0,
            time: 0.0,
            pointer_id: 0,
        }
    }

    fn input(id: Id, value: &str) -> CanvasEvent {
        event(EventType::Input, id, value)
    }

    fn value_of(commands: &[Command], target: &Id) -> Option<String> {
        commands.iter().find_map(|command| match command {
            Command::SetValue { id, value } | Command::SetText { id, value } if id == target => {
                Some(value.clone())
            }
            _ => None,
        })
    }

    fn style_of(commands: &[Command], target: &Id, property: crate::js_client::StyleProperty) -> Option<String> {
        commands.iter().find_map(|command| match command {
            Command::SetStyle { id, property: p, value: StyleValue::Text(value) }
                if id == target && *p == property =>
            {
                Some(value.clone())
            }
            _ => None,
        })
    }

    // dom::Id -> element id in the html (init.js: segment = tag[-n], joined by "_")
    fn element_id(id: &Id) -> String {
        let segment = |segment: &crate::js_client::dom::Segment| {
            let tag = format!("{:?}", segment.tag).to_lowercase();
            match segment.n {
                Some(n) => format!("{tag}-{n}"),
                None => tag,
            }
        };
        id.0.iter().map(segment).collect::<Vec<_>>().join("_")
    }

    #[test]
    fn hsl_and_rgb_agree_on_the_primaries_and_round_trip_every_byte_color() {
        assert_eq!(to_rgb([0.0, 100.0, 50.0]), [255, 0, 0]);
        assert_eq!(to_rgb([120.0, 100.0, 50.0]), [0, 255, 0]);
        assert_eq!(to_rgb([240.0, 100.0, 50.0]), [0, 0, 255]);
        assert_eq!(to_rgb([0.0, 0.0, 100.0]), [255, 255, 255]);
        assert_eq!(from_rgb([99, 102, 241]).map(libm::round), [239.0, 84.0, 67.0]);

        for r in (0..=255).step_by(15) {
            for g in (0..=255).step_by(15) {
                for b in (0..=255).step_by(15) {
                    assert_eq!(to_rgb(from_rgb([r, g, b])), [r, g, b]);
                }
            }
        }
    }

    #[test]
    fn hex_input_replaces_the_color_and_redraws_every_view() {
        let mut picker = ColorPicker::new();
        let commands = picker.process_canvas(&input(hex_input(), "#ff0000"));

        assert_eq!(style_of(&commands, &preview(), Background).unwrap(), "#FF0000");
        assert_eq!(value_of(&commands, &hex_input()).unwrap(), "#FF0000");
        assert_eq!(style_of(&commands, &hex_input(), Color).unwrap(), "#FFFFFF");
        assert_eq!(value_of(&commands, &slider(0, Tag::Input)).unwrap(), "0");
        assert_eq!(value_of(&commands, &slider(1, Tag::Output)).unwrap(), "100%");
        assert_eq!(value_of(&commands, &slider(2, Tag::Output)).unwrap(), "50%");
        assert_eq!(
            style_of(&commands, &slider(1, Tag::Input), Background).unwrap(),
            "linear-gradient(to right, #808080, #FF0000)"
        );
    }

    #[test]
    fn malformed_hex_changes_nothing() {
        let mut picker = ColorPicker::new();
        let before = picker.hsl;
        for value in ["", "#", "ff0000", "#ff000", "#ff00000", "#gg0000", "#+f0000", "# f0000"] {
            assert!(picker.process_canvas(&input(hex_input(), value)).is_empty(), "{value:?}");
        }
        assert_eq!(picker.hsl, before);
    }

    #[test]
    fn each_slider_moves_only_its_own_channel() {
        for (n, max) in HSL_MAX.into_iter().enumerate() {
            let mut picker = ColorPicker::new();
            let before = picker.hsl;
            picker.process_canvas(&input(slider(n, Tag::Input), "10"));
            for channel in 0..3 {
                let expected = if channel == n { 10.0 } else { before[channel] };
                assert_eq!(picker.hsl[channel], expected, "slider {n} channel {channel}");
            }

            // out of range is clamped, not trusted
            picker.process_canvas(&input(slider(n, Tag::Input), "1e9"));
            assert_eq!(picker.hsl[n], max);
            picker.process_canvas(&input(slider(n, Tag::Input), "-5"));
            assert_eq!(picker.hsl[n], 0.0);

            let rejected = picker.hsl;
            for value in ["", "abc", "NaN", "inf"] {
                assert!(picker.process_canvas(&input(slider(n, Tag::Input), value)).is_empty());
            }
            assert_eq!(picker.hsl, rejected);
        }
    }

    #[test]
    fn a_dragged_to_gray_slider_keeps_the_hue_it_had() {
        let mut picker = ColorPicker::new();
        let hue = picker.hsl[0];
        picker.process_canvas(&input(slider(2, Tag::Input), "0"));
        picker.process_canvas(&input(slider(2, Tag::Input), "50"));
        assert_eq!(picker.hsl[0], hue);
    }

    #[test]
    fn swatch_click_keeps_hue_and_saturation_and_takes_the_swatch_lightness() {
        let mut picker = ColorPicker::new();
        picker.process_canvas(&input(hex_input(), "#336699"));
        let [hue, saturation, _] = picker.hsl;

        let drawn = picker.draw();
        let swatch_hex = value_of(&drawn, &swatch(6, Tag::Span)).unwrap();
        let commands = picker.process_canvas(&event(EventType::Click, swatch(6, Tag::Input), ""));

        assert_eq!(picker.hsl[0..2], [hue, saturation]);
        assert_eq!(picker.hsl[2], 90.0);
        assert_eq!(value_of(&commands, &hex_input()).unwrap(), swatch_hex);
    }

    #[test]
    fn unrelated_events_are_ignored() {
        let mut picker = ColorPicker::new();
        let before = picker.hsl;
        assert!(picker.process_canvas(&event(EventType::Click, hex_input(), "")).is_empty());
        assert!(picker.process_canvas(&event(EventType::Click, swatch(7, Tag::Input), "")).is_empty());
        assert!(picker.process_canvas(&input(swatch(0, Tag::Input), "#ff0000")).is_empty());
        assert!(picker.process_canvas(&event(EventType::Change, hex_input(), "#ff0000")).is_empty());
        assert_eq!(picker.hsl, before);
    }

    fn checked(commands: &[Command]) -> Vec<bool> {
        (0..7)
            .map(|n| {
                commands.iter().any(|command| {
                    matches!(command, Command::SetAttribute { id, attribute: Checked, .. }
                        if *id == swatch(n, Tag::Input))
                })
            })
            .collect()
    }

    #[test]
    fn only_the_swatch_at_the_current_lightness_is_checked_and_the_rest_are_cleared() {
        let mut picker = ColorPicker::new();

        let commands = picker.process_canvas(&input(slider(2, Tag::Input), "35"));
        assert_eq!(checked(&commands), [false, false, true, false, false, false, false]);
        let cleared = commands
            .iter()
            .filter(|command| matches!(command, Command::RemoveAttribute { attribute: Checked, .. }))
            .count();
        assert_eq!(cleared, 6);

        // lightness between swatches: none is checked, and the old one is cleared
        let commands = picker.process_canvas(&input(slider(2, Tag::Input), "40"));
        assert_eq!(checked(&commands), [false; 7]);

        // a click checks its own swatch; the browser-side check follows the lightness
        let commands = picker.process_canvas(&event(EventType::Click, swatch(0, Tag::Input), ""));
        assert_eq!(checked(&commands), [true, false, false, false, false, false, false]);
    }

    #[test]
    fn contrast_ink_follows_the_background() {
        assert_eq!(ink([255, 255, 0]), "#000000");
        assert_eq!(ink([0, 0, 128]), "#FFFFFF");
    }

    #[test]
    fn a_wire_frame_from_init_js_drives_the_picker_and_the_reply_encodes() {
        let mut frame = Vec::new();
        frame.push(EVENT_CANVAS);
        frame.push(7); // EventType::Input (init.js EVENT_TYPES)
        hex_input().encode(&mut frame);
        frame.push(0);
        frame.push(0);
        put_str(&mut frame, "#00FF00");
        for _ in 0..4 {
            put_f32(&mut frame, 0.0);
        }
        frame.extend_from_slice(&0f64.to_le_bytes());
        put_u32(&mut frame, 0);

        let Some(Event::Canvas(received)) = decode_event(&frame) else { unreachable!() };
        assert_eq!(received.event_type, EventType::Input);
        let commands = ColorPicker::new().process_canvas(&received);
        assert_eq!(style_of(&commands, &preview(), Background).unwrap(), "#00FF00");

        for command in &commands {
            let mut out = Vec::new();
            encode_command(&mut out, command);
            assert!(!out.is_empty());
        }
    }

    #[test]
    fn every_command_targets_an_element_in_the_html() {
        let html = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/reference/color_picker.html"))
            .unwrap();
        for command in ColorPicker::new().draw() {
            let (Command::SetText { id, .. }
            | Command::SetValue { id, .. }
            | Command::SetStyle { id, .. }
            | Command::SetAttribute { id, .. }
            | Command::RemoveAttribute { id, .. }) = command
            else {
                unreachable!()
            };
            let element = format!("id=\"{}\"", element_id(&id));
            assert!(html.contains(&element), "{element} is not in color_picker.html");
        }
        // events come from these too
        for id in [
            slider(0, Tag::Input),
            slider(2, Tag::Input),
            hex_input(),
            swatch(0, Tag::Input),
            swatch(6, Tag::Input),
        ] {
            assert!(html.contains(&format!("id=\"{}\"", element_id(&id))));
        }
    }
}
