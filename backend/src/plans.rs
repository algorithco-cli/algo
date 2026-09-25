//! Plan catalog: pro / max / team (+ free). Prices mirror web Pricing page.
//!
//! Plans are BUNDLES of entitlements: handlers never branch on tier strings,
//! they check `features.*` / limits from `resolve_entitlement`. Amounts are
//! per-seat per-month in minor units (cents); `annual_cents` is the monthly
//! rate when billed annually (matches "$10/mo billed annually").

use serde::{Deserialize, Serialize};

pub const TIER_FREE: &str = "free";
pub const TIER_PRO: &str = "pro";
pub const TIER_MAX: &str = "max";
pub const TIER_TEAM: &str = "team";

/// Paid tiers in upgrade order.
pub const PAID_TIERS: &[&str] = &[TIER_PRO, TIER_MAX, TIER_TEAM];

/// Trial length for self-serve subscriptions (no card required).
pub const TRIAL_DAYS: i64 = 14;
/// Billing period length (MVP: fixed 30d for both cycles; annual only reprices).
pub const PERIOD_DAYS: i64 = 30;
/// Past-due grace with full access before paid features revoke.
pub const GRACE_DAYS: i64 = 14;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureSet {
    pub signed_bundles: bool,
    pub dry_run: bool,
    pub dashboard: bool,
    pub per_user_stats: bool,
    pub oauth_org: bool,
    pub siem_export: bool,
    pub sso_scim: bool,
}

impl FeatureSet {
    pub fn free() -> Self {
        Self {
            signed_bundles: false,
            dry_run: false,
            dashboard: false,
            per_user_stats: false,
            oauth_org: false,
            siem_export: false,
            sso_scim: false,
        }
    }

    /// Look up one feature by API name. Unknown names → None (fail-closed:
    /// callers deny unknown features rather than allowing them).
    pub fn get(&self, feature: &str) -> Option<bool> {
        match feature {
            "signed_bundles" => Some(self.signed_bundles),
            "dry_run" => Some(self.dry_run),
            "dashboard" => Some(self.dashboard),
            "per_user_stats" => Some(self.per_user_stats),
            "oauth_org" => Some(self.oauth_org),
            "siem_export" => Some(self.siem_export),
            "sso_scim" => Some(self.sso_scim),
            _ => None,
        }
    }

    /// Cheapest paid tier that includes `feature` (for upgrade hints).
    pub fn min_tier_for(feature: &str) -> Option<&'static str> {
        for tier in PAID_TIERS {
            if let Some(plan) = plan_for(tier) {
                if plan.features.get(feature) == Some(true) {
                    return Some(tier);
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Plan {
    pub tier: String,
    pub name: String,
    pub monthly_cents: i64,
    pub annual_cents: i64,
    pub seats_included: i64,
    /// Hard seat cap; -1 = unlimited.
    pub max_seats: i64,
    pub rate_multiplier: i64,
    pub retention_days: i64,
    pub features: FeatureSet,
}

fn pro_features() -> FeatureSet {
    FeatureSet {
        signed_bundles: true,
        dry_run: true,
        dashboard: true,
        per_user_stats: true,
        oauth_org: false,
        siem_export: false,
        sso_scim: false,
    }
}

/// Upgrade rank: free < pro < max < team. Unknown → 0 (sorts as free).
pub fn rank(tier: &str) -> u8 {
    match tier {
        TIER_PRO => 1,
        TIER_MAX => 2,
        TIER_TEAM => 3,
        _ => 0,
    }
}

pub fn is_paid_tier(tier: &str) -> bool {
    matches!(tier, TIER_PRO | TIER_MAX | TIER_TEAM)
}

pub fn plan_for(tier: &str) -> Option<Plan> {
    match tier {
        TIER_PRO => Some(Plan {
            tier: TIER_PRO.to_string(),
            name: "Pro".to_string(),
            monthly_cents: 1200,
            annual_cents: 1000,
            seats_included: 1,
            max_seats: 25,
            rate_multiplier: 1,
            retention_days: 7,
            features: pro_features(),
        }),
        TIER_MAX => Some(Plan {
            tier: TIER_MAX.to_string(),
            name: "Max".to_string(),
            monthly_cents: 2900,
            annual_cents: 2400,
            seats_included: 1,
            max_seats: 100,
            rate_multiplier: 3,
            retention_days: 30,
            features: FeatureSet {
                oauth_org: true,
                ..pro_features()
            },
        }),
        TIER_TEAM => Some(Plan {
            tier: TIER_TEAM.to_string(),
            name: "Team".to_string(),
            monthly_cents: 4900,
            annual_cents: 4100,
            seats_included: 1,
            max_seats: -1,
            rate_multiplier: 10,
            retention_days: 365,
            features: FeatureSet {
                signed_bundles: true,
                dry_run: true,
                dashboard: true,
                per_user_stats: true,
                oauth_org: true,
                siem_export: true,
                sso_scim: true,
            },
        }),
        _ => None,
    }
}

pub fn catalog() -> Vec<Plan> {
    [TIER_PRO, TIER_MAX, TIER_TEAM]
        .into_iter()
        .filter_map(plan_for)
        .collect()
}

/// Per-seat rate for a cycle ("monthly" | "annual").
pub fn rate_for(plan: &Plan, cycle: &str) -> Option<i64> {
    match cycle {
        "monthly" => Some(plan.monthly_cents),
        "annual" => Some(plan.annual_cents),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;

    #[test]
    fn catalog_matches_pricing_page() {
        let _guard = test_sync::lock();
        let plans = catalog();
        assert_eq!(plans.len(), 3);
        let pro = &plans[0];
        assert_eq!((pro.monthly_cents, pro.annual_cents), (1200, 1000));
        assert_eq!(
            (plans[1].monthly_cents, plans[1].annual_cents),
            (2900, 2400)
        );
        assert_eq!(
            (plans[2].monthly_cents, plans[2].annual_cents),
            (4900, 4100)
        );
        // Compare-table parity: signed/dry-run from pro, siem/sso team-only.
        assert!(pro.features.dry_run && pro.features.signed_bundles);
        assert!(!pro.features.siem_export && !pro.features.sso_scim && !pro.features.oauth_org);
        assert!(plans[1].features.oauth_org && !plans[1].features.siem_export);
        assert!(plans[2].features.siem_export && plans[2].features.sso_scim);
    }

    #[test]
    fn unknown_feature_denies_closed() {
        let _guard = test_sync::lock();
        let pro = plan_for(TIER_PRO).unwrap();
        assert_eq!(pro.features.get("nope"), None);
        assert_eq!(FeatureSet::min_tier_for("nope"), None);
        assert_eq!(FeatureSet::min_tier_for("dry_run"), Some(TIER_PRO));
        assert_eq!(FeatureSet::min_tier_for("siem_export"), Some(TIER_TEAM));
    }
}
