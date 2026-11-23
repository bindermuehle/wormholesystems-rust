use super::common::LocalizedString;
use serde::Deserialize;

/// `skins.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "allowCCPDevs")]
    pub allow_ccp_devs: bool,
    pub internal_name: String,
    #[serde(rename = "skinMaterialID")]
    pub skin_material_id: i64,
    pub types: Vec<i32>,
    pub visible_serenity: bool,
    pub visible_tranquility: bool,
    pub is_structure_skin: Option<bool>,
}

/// `skinMaterials.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinMaterial {
    #[serde(rename = "_key")]
    pub id: i64,
    pub display_name: Option<LocalizedString>,
    #[serde(rename = "materialSetID")]
    pub material_set_id: i64,
}

/// `skinLicenses.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinLicense {
    #[serde(rename = "_key")]
    pub id: i64,
    pub duration: i32,
    /// The same as the key. Builds from 3698585 on leave it out.
    #[serde(rename = "licenseTypeID")]
    pub license_type_id: Option<i64>,
    #[serde(rename = "skinID")]
    pub skin_id: i64,
    pub is_single_use: Option<bool>,
}

/// Element of `skinrComponents.associatedTypeIds`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrComponentAssociatedTypeId {
    pub license_uses_granted: i32,
    #[serde(rename = "typeID")]
    pub type_id: i64,
}

/// `skinrComponents.sequenceBinder`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrComponentSequenceBinder {
    pub count: i32,
    #[serde(rename = "itemTypeID")]
    pub item_type_id: i64,
}

/// `skinrComponents.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrComponent {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "associatedTypeIds")]
    pub associated_type_ids: Vec<SkinrComponentAssociatedTypeId>,
    pub category: i32,
    pub finish: String,
    pub icon_file: String,
    pub name: LocalizedString,
    pub projection_type_u: String,
    pub projection_type_v: String,
    pub published: bool,
    pub rarity: i32,
    pub resource_file: String,
    pub sequence_binder: SkinrComponentSequenceBinder,
}

/// `skinrComponentCategories.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrComponentCategory {
    #[serde(rename = "_key")]
    pub id: i64,
    pub name: String,
}

/// `skinrComponentRarities.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrComponentRarity {
    #[serde(rename = "_key")]
    pub id: i64,
    pub name: LocalizedString,
    pub rank: i32,
}

/// Element of `skinrComponentPointValues._value`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrComponentPointValueEntry {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "_value")]
    pub value: i32,
}

/// `skinrComponentPointValues.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrComponentPointValue {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "_value")]
    pub value: Vec<SkinrComponentPointValueEntry>,
}

/// `skinrSlots.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrSlot {
    #[serde(rename = "_key")]
    pub id: i64,
    pub allowed_design_component_categories: Vec<i32>,
    pub category: i32,
    pub name: LocalizedString,
}

/// `skinrSlotCategories.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrSlotCategory {
    #[serde(rename = "_key")]
    pub id: i64,
    pub name: String,
}

/// `skinrSlotNames.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrSlotName {
    #[serde(rename = "_key")]
    pub id: i64,
    pub name: String,
}

/// `skinrSlotConfigurations.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrSlotConfiguration {
    #[serde(rename = "_key")]
    pub id: i64,
    pub allow_all_ships: Option<bool>,
    pub config: Option<Vec<i32>>,
    pub name: String,
    pub priority: i32,
    pub ships: Option<Vec<i32>>,
}

/// Element of `skinrTierThresholds._value`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrTierThresholdEntry {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "_value")]
    pub value: i32,
}

/// `skinrTierThresholds.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinrTierThreshold {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "_value")]
    pub value: Vec<SkinrTierThresholdEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_skins() {
        let Some(rows) = crate::sde::parse_sample::<Skin>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skin_materials() {
        let Some(rows) = crate::sde::parse_sample::<SkinMaterial>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skin_licenses() {
        let Some(rows) = crate::sde::parse_sample::<SkinLicense>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_components() {
        let Some(rows) = crate::sde::parse_sample::<SkinrComponent>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_component_categories() {
        let Some(rows) = crate::sde::parse_sample::<SkinrComponentCategory>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_component_rarities() {
        let Some(rows) = crate::sde::parse_sample::<SkinrComponentRarity>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_component_point_values() {
        let Some(rows) = crate::sde::parse_sample::<SkinrComponentPointValue>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_slots() {
        let Some(rows) = crate::sde::parse_sample::<SkinrSlot>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_slot_categories() {
        let Some(rows) = crate::sde::parse_sample::<SkinrSlotCategory>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_slot_names() {
        let Some(rows) = crate::sde::parse_sample::<SkinrSlotName>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_slot_configurations() {
        let Some(rows) = crate::sde::parse_sample::<SkinrSlotConfiguration>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_skinr_tier_thresholds() {
        let Some(rows) = crate::sde::parse_sample::<SkinrTierThreshold>() else {
            return;
        };
        assert!(!rows.is_empty());
    }
}
