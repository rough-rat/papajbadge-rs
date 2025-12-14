#![no_std]

//! LED watchface layout handling.
//!
//! This crate defines a JSON-driven format for mapping logical watchface entities
//! (rings, bars, etc.) to the physical LED indices controlled by firmware.

use heapless::{String, Vec};
use serde::{Deserialize, Serialize};
use serde_json_core::de::Error as JsonError;

pub type LayoutResult<T> = core::result::Result<T, LayoutError>;

#[derive(Debug)]
pub enum LayoutError {
    Json(JsonError),
    Binary(postcard::Error),
}

impl From<JsonError> for LayoutError {
    fn from(err: JsonError) -> Self {
        Self::Json(err)
    }
}

impl From<postcard::Error> for LayoutError {
    fn from(err: postcard::Error) -> Self {
        Self::Binary(err)
    }
}

/// Maximum UTF-8 length used for any name field parsed from JSON.
pub const NAME_CAP: usize = 32;
/// Maximum UTF-8 length for human readable notes.
pub const NOTES_CAP: usize = 96;

/// Parsed watchface layout description.
#[derive(Debug, Deserialize, Serialize)]
pub struct Layout<const MAX_LEDS: usize, const MAX_ENTITIES: usize, const MAX_ENTITY_MAP: usize> {
    #[serde(default)]
    pub watchface: Option<WatchfaceInfo>,
    pub leds: Vec<Led, MAX_LEDS>,
    pub entities: Vec<Entity<MAX_ENTITY_MAP>, MAX_ENTITIES>,
}

impl<const MAX_LEDS: usize, const MAX_ENTITIES: usize, const MAX_ENTITY_MAP: usize>
    Layout<MAX_LEDS, MAX_ENTITIES, MAX_ENTITY_MAP>
{
    /// Parse a layout description from JSON.
    pub fn from_json(json: &[u8]) -> LayoutResult<Self> {
        let (layout, _consumed) = serde_json_core::from_slice(json).map_err(LayoutError::from)?;
        Ok(layout)
    }

    /// Parse a layout from postcard-encoded bytes.
    pub fn from_bytes(bytes: &[u8]) -> LayoutResult<Self> {
        postcard::from_bytes(bytes).map_err(LayoutError::from)
    }

    /// Get entity by name.
    pub fn entity(&self, name: &str) -> Option<&Entity<MAX_ENTITY_MAP>> {
        self.entities.iter().find(|entity| entity.name.as_str() == name)
    }

    /// Convenience helper returning number of LEDs defined in the layout.
    pub fn led_count(&self) -> usize {
        self.leds.len()
    }
}

/// Metadata about the PCB / watchface.
#[derive(Debug, Deserialize, Serialize)]
pub struct WatchfaceInfo {
    pub name: String<NAME_CAP>,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub notes: String<NOTES_CAP>,
}

/// Physical LED record with location metadata.
#[derive(Debug, Deserialize, Serialize)]
pub struct Led {
    pub index: u16,
    #[serde(default)]
    pub name: String<NAME_CAP>,
    pub x_mm: f32,
    pub y_mm: f32,
}

/// Logical entity type.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Group,
    Bar,
    Ring,
    Matrix,
    #[serde(other)]
    Unknown,
}

/// Logical entity that references one or more LEDs.
#[derive(Debug, Deserialize, Serialize)]
pub struct Entity<const MAX_ENTITY_MAP: usize> {
    pub name: String<NAME_CAP>,
    #[serde(rename = "type")]
    pub kind: EntityType,
    #[serde(default)]
    pub map: Vec<u16, MAX_ENTITY_MAP>,
    #[serde(default)]
    pub params: EntityParams,
}

impl<const MAX_ENTITY_MAP: usize> Entity<MAX_ENTITY_MAP> {
    /// Fill LEDs proportionally to the supplied degrees (0-360).
    ///
    /// The `frame` buffer should contain per-LED brightness values; indices not in this
    /// entity's map are ignored if present.
    pub fn set_degrees(&self, degrees: u16, frame: &mut [u8]) {
        let count = self.map.len();
        if count == 0 {
            return;
        }

        let clamped = core::cmp::min(degrees, 360) as usize;
        let lit = ((clamped * count) + 359) / 360; // round up

        for (idx, led_index) in self.map.iter().enumerate() {
            if let Some(slot) = frame.get_mut(*led_index as usize) {
                *slot = if idx < lit { 0xFF } else { 0x00 };
            }
        }
    }
}

/// Optional type-specific metadata.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct EntityParams {
    #[serde(default)]
    pub orientation: Option<String<NAME_CAP>>,
    #[serde(default)]
    pub radius_mm: Option<f32>,
    #[serde(default)]
    pub clockwise: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_test_layout() {
        let data = include_bytes!("../test/test.json");
        let layout: Layout<48, 4, 48> = Layout::from_json(data).unwrap();
        assert_eq!(layout.led_count(), 48);

        let text = layout.entity("Text").expect("missing Text entity");
        assert_eq!(text.map.len(), 13);

        let wave = layout.entity("Wave").expect("missing Wave entity");
        assert_eq!(wave.map.first(), Some(&0));
        assert_eq!(wave.map.last(), Some(&47));
    }

    #[test]
    fn json_to_binary_roundtrip() {
        let data = include_bytes!("../test/test.json");
        let layout: Layout<48, 4, 48> = Layout::from_json(data).unwrap();

        let mut buf = [0u8; 1024];
        let slice = postcard::to_slice(&layout, &mut buf).unwrap();
        let decoded: Layout<48, 4, 48> = Layout::from_bytes(slice).unwrap();
        assert_eq!(decoded.led_count(), layout.led_count());
    }
}
