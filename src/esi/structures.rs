//! What ESI says about a corporation's structures: the notifications a character received
//! about them, and the list with the fuel timers.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{EsiClient, Result};

/// One in-game notification, as its recipient sees it.
#[derive(Debug, Clone, Deserialize)]
pub struct Notification {
    pub notification_id: i64,
    /// EVE's name for what happened, e.g. `StructureUnderAttack`. A string and not an enum:
    /// CCP adds types, and one we do not know must not fail the whole list.
    #[serde(rename = "type")]
    pub kind: String,
    pub timestamp: DateTime<Utc>,
    /// The details, as a small YAML document. See [`Notification::field`].
    pub text: Option<String>,
}

impl Notification {
    /// A top-level value of the notification's text, under the first of `keys` it has. The
    /// keys are not spelled the same from one notification type to the next
    /// (`solarsystemID` here, `solarSystemID` there), hence several.
    ///
    /// The text is YAML, but only ever a flat map of scalars with the odd list hanging off
    /// it, so this reads the lines instead of pulling in a parser. An anchored value
    /// (`structureID: &id001 1026660410904`) comes back without its anchor.
    pub fn field(&self, keys: &[&str]) -> Option<&str> {
        let text = self.text.as_deref()?;
        keys.iter().find_map(|key| {
            text.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name != *key {
                    return None;
                }
                let value = value.trim();
                let value = match value.strip_prefix('&') {
                    Some(anchored) => anchored.split_once(' ')?.1.trim(),
                    None => value,
                };
                let value = value.trim_matches(|c| c == '\'' || c == '"');
                (!value.is_empty() && value != "null" && value != "~").then_some(value)
            })
        })
    }

    pub fn id(&self, keys: &[&str]) -> Option<i64> {
        self.field(keys)?.parse().ok().filter(|id| *id > 0)
    }

    pub fn number(&self, keys: &[&str]) -> Option<f64> {
        self.field(keys)?
            .parse()
            .ok()
            .filter(|n: &f64| n.is_finite())
    }
}

/// An Upwell structure, as its corporation's list has it.
#[derive(Debug, Clone, Deserialize)]
pub struct CorporationStructure {
    pub structure_id: i64,
    pub corporation_id: i64,
    pub name: Option<String>,
    pub type_id: i64,
    pub system_id: i64,
    /// `shield_vulnerable`, `armor_reinforce`, `hull_reinforce`, `anchoring` and so on.
    pub state: String,
    /// Absent once it has run dry.
    pub fuel_expires: Option<DateTime<Utc>>,
    /// When the current reinforcement or anchoring timer runs out.
    pub state_timer_end: Option<DateTime<Utc>>,
}

impl EsiClient {
    /// The notifications `character_id` received, newest 500. Needs
    /// `esi-characters.read_notifications.v1`. ESI caches the list for ten minutes and
    /// allows a character only a handful of reads in fifteen, so this goes through the
    /// polled path: a `304` costs half a `200`.
    pub async fn character_notifications(
        &self,
        token: &str,
        character_id: i64,
    ) -> Result<Vec<Notification>> {
        self.get_json_polled(&format!("/characters/{character_id}/notifications"), token)
            .await
    }

    /// Every Upwell structure the corporation owns. Needs
    /// `esi-corporations.read_structures.v1` and the Station Manager role in game: a 403
    /// means the character does not have it.
    pub async fn corporation_structures(
        &self,
        token: &str,
        corporation_id: i64,
    ) -> Result<Vec<CorporationStructure>> {
        self.get_json_paged(&format!("/corporations/{corporation_id}/structures"), token)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(text: &str) -> Notification {
        Notification {
            notification_id: 1,
            kind: "StructureUnderAttack".into(),
            timestamp: Utc::now(),
            text: Some(text.into()),
        }
    }

    #[test]
    fn a_field_is_read_from_the_top_level_only() {
        let n = notification(
            "allianceID: 99005338\nallianceLinkData:\n- showinfo\n- 16159\n- 99005338\n\
             allianceName: Pandemic Horde\narmorPercentage: 100.0\ncharID: 2112625428\n\
             corpName: 'Horde Vanguard.'\nshieldPercentage: 94.88\nsolarsystemID: 30004608\n\
             structureID: &id001 1026660410904\nstructureShowInfoData:\n- showinfo\n- 35833\n\
             - *id001\nstructureTypeID: 35833\n",
        );
        assert_eq!(n.id(&["structureID"]), Some(1026660410904));
        assert_eq!(n.id(&["solarSystemID", "solarsystemID"]), Some(30004608));
        assert_eq!(n.field(&["corpName"]), Some("Horde Vanguard."));
        assert_eq!(n.number(&["shieldPercentage"]), Some(94.88));
        assert_eq!(n.id(&["structureTypeID", "typeID"]), Some(35833));
        // A list is not a value, and a key that is not there is nothing.
        assert_eq!(n.field(&["allianceLinkData"]), None);
        assert_eq!(n.field(&["moonID"]), None);
    }

    #[test]
    fn a_missing_or_null_value_is_nothing() {
        let n = notification("aggressorAllianceID: null\naggressorID: 0\nmoonID: 40009081\n");
        assert_eq!(n.id(&["aggressorAllianceID"]), None);
        assert_eq!(n.id(&["aggressorID"]), None);
        assert_eq!(n.id(&["moonID"]), Some(40009081));
        let empty = Notification {
            text: None,
            ..notification("")
        };
        assert_eq!(empty.field(&["moonID"]), None);
    }
}
