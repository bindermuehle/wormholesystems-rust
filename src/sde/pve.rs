//! PvE-related SDE entities: missions, dungeons, epic arcs, military campaigns,
//! mercenary tactical operations, freelance job schemas and sovereignty upgrades.

use super::common::LocalizedString;
use serde::Deserialize;

/// `missions.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mission {
    #[serde(rename = "_key")]
    pub id: i64,
    pub has_standing_rewards: bool,
    pub kill_mission: Option<MissionKillMission>,
    pub messages: Option<Vec<MissionMessage>>,
    pub name: LocalizedString,
    pub expiration_time: Option<i32>,
    #[serde(rename = "factionID")]
    pub faction_id: Option<i64>,
    pub courier_mission: Option<MissionCourierMission>,
    pub mission_rewards: Option<MissionRewards>,
    #[serde(rename = "corporationID")]
    pub corporation_id: Option<i64>,
    pub initial_agent_gift_quantity: Option<i32>,
    #[serde(rename = "initialAgentGiftTypeID")]
    pub initial_agent_gift_type_id: Option<i64>,
    pub extra_standings: Option<Vec<MissionExtraStanding>>,
    #[serde(rename = "agentTypeID")]
    pub agent_type_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionKillMission {
    #[serde(rename = "dungeonID")]
    pub dungeon_id: Option<i64>,
    pub objective_quantity: Option<i32>,
    #[serde(rename = "objectiveTypeID")]
    pub objective_type_id: Option<i64>,
    pub drop_item_in_mission_container: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionMessage {
    #[serde(rename = "_key")]
    pub id: String,
    #[serde(flatten)]
    pub text: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionCourierMission {
    pub objective_quantity: i32,
    pub objective_singleton: bool,
    #[serde(rename = "objectiveTypeID")]
    pub objective_type_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionRewards {
    pub bonus_reward: Option<MissionRewardItem>,
    pub bonus_time_interval: Option<i32>,
    pub reward: Option<MissionRewardItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionRewardItem {
    pub reward_quantity: Option<i32>,
    #[serde(rename = "rewardTypeID")]
    pub reward_type_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionExtraStanding {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "_value")]
    pub value: f64,
}

/// `dungeons.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dungeon {
    #[serde(rename = "_key")]
    pub id: i64,
    pub allowed_ships_list: Option<Vec<i32>>,
    #[serde(rename = "archetypeID")]
    pub archetype_id: i64,
    pub description: Option<LocalizedString>,
    #[serde(rename = "factionID")]
    pub faction_id: Option<i64>,
    pub name: LocalizedString,
    pub gameplay_description: Option<LocalizedString>,
}

/// `epicArcs.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicArc {
    #[serde(rename = "_key")]
    pub id: i64,
    pub arc_restart_interval: i32,
    #[serde(rename = "factionID")]
    pub faction_id: Option<i64>,
    #[serde(rename = "iconID")]
    pub icon_id: i64,
    pub missions: Vec<EpicArcMission>,
    pub name: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicArcMission {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "agentID")]
    pub agent_id: i64,
    #[serde(rename = "failMissionID")]
    pub fail_mission_id: Option<i64>,
    pub next_missions: Option<Vec<i32>>,
}

/// `militaryCampaigns.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilitaryCampaign {
    #[serde(rename = "_key")]
    pub id: String,
    pub annotations: MilitaryCampaignAnnotations,
    pub issuer: MilitaryCampaignIssuer,
    pub subtitle: LocalizedString,
    pub target_progress: i64,
    pub title: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilitaryCampaignIssuer {
    #[serde(rename = "factionID")]
    pub faction_id: i64,
}

/// Nested `annotations` object on a [`MilitaryCampaign`]: UI asset paths, localized copy
/// blocks and a focus entity id.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilitaryCampaignAnnotations {
    #[serde(rename = "aoCampaignCardButtonImage")]
    pub ao_campaign_card_button_image: String,
    #[serde(rename = "backgroundVideoLoop")]
    pub background_video_loop: String,
    #[serde(rename = "briefingBackground")]
    pub briefing_background: String,
    #[serde(rename = "briefingFailureDescription")]
    pub briefing_failure_description: LocalizedString,
    #[serde(rename = "briefingFailureHeader")]
    pub briefing_failure_header: LocalizedString,
    #[serde(rename = "briefingFinalWords")]
    pub briefing_final_words: LocalizedString,
    #[serde(rename = "briefingForeground")]
    pub briefing_foreground: String,
    #[serde(rename = "briefingGoalDescription")]
    pub briefing_goal_description: LocalizedString,
    #[serde(rename = "briefingHeader")]
    pub briefing_header: LocalizedString,
    #[serde(rename = "briefingMiddleground")]
    pub briefing_middleground: String,
    #[serde(rename = "briefingSuccessDescription")]
    pub briefing_success_description: LocalizedString,
    #[serde(rename = "briefingSuccessHeader")]
    pub briefing_success_header: LocalizedString,
    #[serde(rename = "campaignSet")]
    pub campaign_set: String,
    #[serde(rename = "dashboardAmbientBackground")]
    pub dashboard_ambient_background: String,
    #[serde(rename = "dashboardBackground")]
    pub dashboard_background: String,
    #[serde(rename = "dashboardForeground")]
    pub dashboard_foreground: String,
    #[serde(rename = "dashboardMiddleground")]
    pub dashboard_middleground: String,
    #[serde(rename = "finishedCampaignEnded")]
    pub finished_campaign_ended: LocalizedString,
    #[serde(rename = "finishedFailureDescription")]
    pub finished_failure_description: LocalizedString,
    #[serde(rename = "finishedResolutionStateFailure")]
    pub finished_resolution_state_failure: LocalizedString,
    #[serde(rename = "finishedResolutionStateSuccess")]
    pub finished_resolution_state_success: LocalizedString,
    #[serde(rename = "finishedSuccessDescription")]
    pub finished_success_description: LocalizedString,
    #[serde(rename = "foregroundVideoIntro")]
    pub foreground_video_intro: String,
    #[serde(rename = "foregroundVideoLoop")]
    pub foreground_video_loop: String,
    #[serde(rename = "foregroundVideoOutro")]
    pub foreground_video_outro: String,
    #[serde(rename = "mapFocusEntityID")]
    pub map_focus_entity_id: i64,
    #[serde(rename = "mapHeader")]
    pub map_header: LocalizedString,
    #[serde(rename = "mapSection1Paragraph")]
    pub map_section1_paragraph: LocalizedString,
    #[serde(rename = "mapSection1Title")]
    pub map_section1_title: LocalizedString,
    #[serde(rename = "mapSection2Paragraph")]
    pub map_section2_paragraph: LocalizedString,
    #[serde(rename = "mapSection2Title")]
    pub map_section2_title: LocalizedString,
    #[serde(rename = "mapSection3Paragraph")]
    pub map_section3_paragraph: LocalizedString,
    #[serde(rename = "mapSection3Title")]
    pub map_section3_title: LocalizedString,
    #[serde(rename = "mapSubheader")]
    pub map_subheader: LocalizedString,
    #[serde(rename = "mapTitle")]
    pub map_title: LocalizedString,
    #[serde(rename = "middlegroundVideoIntro")]
    pub middleground_video_intro: String,
    #[serde(rename = "middlegroundVideoLoop")]
    pub middleground_video_loop: String,
    #[serde(rename = "middlegroundVideoOutro")]
    pub middleground_video_outro: String,
    #[serde(rename = "presentingCharacterName")]
    pub presenting_character_name: LocalizedString,
    #[serde(rename = "presentingCharacterSubtitle")]
    pub presenting_character_subtitle: LocalizedString,
    #[serde(rename = "presentingCharacterTexturePath")]
    pub presenting_character_texture_path: String,
    pub race: String,
    #[serde(rename = "themePack")]
    pub theme_pack: String,
    #[serde(rename = "towCampaignCardButtonImage")]
    pub tow_campaign_card_button_image: String,
}

/// `militaryCampaignObjectives.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilitaryCampaignObjective {
    #[serde(rename = "_key")]
    pub id: String,
    #[serde(rename = "campaignID")]
    pub campaign_id: String,
    pub career_path: String,
    pub content_tags: Vec<String>,
    pub contribution_method_configuration: ObjectiveContributionMethod,
    pub issuer: ObjectiveCorporationIssuer,
    pub max_progress_per_participant: i64,
    #[serde(rename = "presentingCharacterID")]
    pub presenting_character_id: i64,
    pub rewards: ObjectiveRewards,
    pub subtitle: LocalizedString,
    pub target_progress: i64,
    pub title: LocalizedString,
    pub annotations: Option<ObjectiveAnnotations>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveContributionMethod {
    pub name: String,
    pub parameters: Vec<ObjectiveParameter>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveParameter {
    pub key: String,
    pub matcher: ObjectiveMatcher,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveMatcher {
    pub values: Vec<ObjectiveMatcherValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveMatcherValue {
    pub value_type: String,
    pub values: Option<Vec<String>>,
}

/// A `{ corporationID }` issuer used by objectives and currency rewards.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveCorporationIssuer {
    #[serde(rename = "corporationID")]
    pub corporation_id: i64,
}

/// A `{ factionID }` issuer used by standing rewards.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveFactionIssuer {
    #[serde(rename = "factionID")]
    pub faction_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveRewards {
    pub isk: ObjectiveCurrencyReward,
    pub lp: ObjectiveCurrencyReward,
    pub standing: Option<ObjectiveStandingReward>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveCurrencyReward {
    pub amount_per_interval: i32,
    pub issuer: ObjectiveCorporationIssuer,
    pub progress_interval: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveStandingReward {
    pub gain_percent_per_interval: f64,
    pub issuer: ObjectiveFactionIssuer,
    pub progress_interval: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveAnnotations {
    #[serde(rename = "requiredEnlistmentWithFactionID")]
    pub required_enlistment_with_faction_id: i64,
    pub restriction_tooltip: LocalizedString,
    pub warning1: LocalizedString,
    pub warning2: Option<LocalizedString>,
}

/// `mercenaryTacticalOperations.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MercenaryTacticalOperation {
    #[serde(rename = "_key")]
    pub id: i64,
    pub anarchy_impact: i32,
    pub description: LocalizedString,
    pub development_impact: i32,
    /// Gone from builds 3698585 on.
    #[serde(rename = "dungeonID")]
    pub dungeon_id: Option<i64>,
    pub infomorph_bonus: i32,
    pub name: LocalizedString,
}

/// `freelanceJobSchemas.jsonl`
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceJobSchema {
    #[serde(rename = "_key")]
    pub id: i64,
    #[serde(rename = "_value")]
    pub value: Vec<FreelanceJob>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceJob {
    #[serde(rename = "_key")]
    pub id: String,
    pub content_tags: Vec<String>,
    pub contribution_multiplier: Option<FreelanceContributionMultiplier>,
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub max_contributions_per_participant: FreelanceMetaField,
    pub max_progress_per_contribution: Option<FreelanceMetaField>,
    pub parameters: Vec<FreelanceParameter>,
    pub progress_description: LocalizedString,
    pub reward_description: LocalizedString,
    pub target_description: LocalizedString,
    pub title: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceMetaField {
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub title: LocalizedString,
    pub unset_description: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceContributionMultiplier {
    pub default_value: f64,
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub max_value: f64,
    pub min_value: f64,
    pub title: LocalizedString,
    pub unset_description: LocalizedString,
}

/// One entry in a [`FreelanceJob`]'s `parameters` array. Carries exactly one of
/// `matcher`, `boolean` or `itemDelivery` alongside its key.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceParameter {
    #[serde(rename = "_key")]
    pub id: String,
    pub matcher: Option<FreelanceMatcher>,
    pub boolean: Option<FreelanceBoolean>,
    pub item_delivery: Option<FreelanceItemDelivery>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceMatcher {
    pub accepted_value_types: Vec<String>,
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub max_entries: i32,
    pub optional: bool,
    pub title: LocalizedString,
    #[serde(rename = "type")]
    pub kind: String,
    pub unset_description: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceBoolean {
    pub choice_label: LocalizedString,
    pub default: bool,
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub option_false: FreelanceBooleanOption,
    pub option_true: FreelanceBooleanOption,
    pub title: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceBooleanOption {
    pub description: LocalizedString,
    pub title: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceItemDelivery {
    pub delivery_location: FreelanceDeliveryLocation,
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub inventory_type: FreelanceInventoryType,
    pub title: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceDeliveryLocation {
    pub accepted_value_types: Vec<String>,
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub max_entries: i32,
    pub title: LocalizedString,
    pub unset_description: LocalizedString,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelanceInventoryType {
    pub accepted_value_types: Vec<String>,
    pub description: LocalizedString,
    #[serde(rename = "iconID")]
    pub icon_id: String,
    pub title: LocalizedString,
    pub unset_description: LocalizedString,
}

/// `sovereigntyUpgrades.jsonl`. Unlike the rest of the SDE, this file uses snake_case keys.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SovereigntyUpgrade {
    #[serde(rename = "_key")]
    pub id: i64,
    pub fuel: Option<SovereigntyUpgradeFuel>,
    #[serde(rename = "mutually_exclusive_group")]
    pub mutually_exclusive_group: String,
    #[serde(rename = "power_allocation")]
    pub power_allocation: Option<i32>,
    #[serde(rename = "workforce_allocation")]
    pub workforce_allocation: Option<i32>,
    #[serde(rename = "power_production")]
    pub power_production: Option<i32>,
    #[serde(rename = "workforce_production")]
    pub workforce_production: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SovereigntyUpgradeFuel {
    #[serde(rename = "hourly_upkeep")]
    pub hourly_upkeep: i32,
    #[serde(rename = "startup_cost")]
    pub startup_cost: i32,
    #[serde(rename = "type_id")]
    pub type_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_missions() {
        let Some(rows) = crate::sde::parse_sample::<Mission>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_dungeons() {
        let Some(rows) = crate::sde::parse_sample::<Dungeon>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_epic_arcs() {
        let Some(rows) = crate::sde::parse_sample::<EpicArc>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_military_campaigns() {
        let Some(rows) = crate::sde::parse_sample::<MilitaryCampaign>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_military_campaign_objectives() {
        let Some(rows) = crate::sde::parse_sample::<MilitaryCampaignObjective>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_mercenary_tactical_operations() {
        let Some(rows) = crate::sde::parse_sample::<MercenaryTacticalOperation>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_freelance_job_schemas() {
        let Some(rows) = crate::sde::parse_sample::<FreelanceJobSchema>() else {
            return;
        };
        assert!(!rows.is_empty());
    }

    #[test]
    fn parses_sovereignty_upgrades() {
        let Some(rows) = crate::sde::parse_sample::<SovereigntyUpgrade>() else {
            return;
        };
        assert!(!rows.is_empty());
    }
}
