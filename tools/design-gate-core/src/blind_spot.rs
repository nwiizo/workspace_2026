use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindSpot {
    pub id: String,
    pub description: String,
    pub description_ja: String,
}

impl BlindSpot {
    pub fn localized_description(&self, japanese: bool) -> &str {
        if japanese {
            &self.description_ja
        } else {
            &self.description
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlindSpotManifest {
    pub blind_spots: Vec<BlindSpot>,
    pub notes: Vec<String>,
    pub notes_ja: Vec<String>,
}

impl BlindSpotManifest {
    pub fn localized_notes(&self, japanese: bool) -> &[String] {
        if japanese {
            &self.notes_ja
        } else {
            &self.notes
        }
    }
}
