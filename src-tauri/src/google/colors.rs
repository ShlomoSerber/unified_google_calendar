//! Modern event palette by `colorId`. See docs/04-fidelidad-visual.md section 7 and
//! docs/research/google-calendar-api.md section 8. `colors.get` returns the classic palette,
//! so the UI never calls it: ids map to these hex values.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteColor {
    pub id: &'static str,
    pub name: &'static str,
    pub bg: &'static str,
}

pub const EVENT_PALETTE: [PaletteColor; 11] = [
    PaletteColor {
        id: "1",
        name: "Lavender",
        bg: "#7986cb",
    },
    PaletteColor {
        id: "2",
        name: "Sage",
        bg: "#33b679",
    },
    PaletteColor {
        id: "3",
        name: "Grape",
        bg: "#8e24aa",
    },
    PaletteColor {
        id: "4",
        name: "Flamingo",
        bg: "#e67c73",
    },
    PaletteColor {
        id: "5",
        name: "Banana",
        bg: "#f6bf26",
    },
    PaletteColor {
        id: "6",
        name: "Tangerine",
        bg: "#f4511e",
    },
    PaletteColor {
        id: "7",
        name: "Peacock",
        bg: "#039be5",
    },
    PaletteColor {
        id: "8",
        name: "Graphite",
        bg: "#616161",
    },
    PaletteColor {
        id: "9",
        name: "Blueberry",
        bg: "#3f51b5",
    },
    PaletteColor {
        id: "10",
        name: "Basil",
        bg: "#0b8043",
    },
    PaletteColor {
        id: "11",
        name: "Tomato",
        bg: "#d50000",
    },
];

pub fn by_id(id: &str) -> Option<&'static PaletteColor> {
    EVENT_PALETTE.iter().find(|c| c.id == id)
}

/// Background of an event: its `colorId` mapped to the modern palette, else the calendar color.
pub fn resolve_bg<'a>(color_id: Option<&str>, calendar_bg: &'a str) -> &'a str {
    match color_id.and_then(by_id) {
        Some(c) => c.bg,
        None => calendar_bg,
    }
}
