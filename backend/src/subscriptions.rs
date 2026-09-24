//! Self-managed subscriptions (no MoR, no payment secrets).
//!
//! Lifecycle (Stripe-aligned subset):
//! create(trialing, 14d, no card) → activate (manual/out-of-band payment) →
//! active ⇄ change (upgrade immediate, downgrade at period end) →
//! cancel (at period end by default) → canceled (access until period end).
//! `past_due`/`unpaid` exist for the future MoR webhook; `expired` is derived
//! lazily when a trial lapses. Effective status is ALWAYS resolved against
//! the clock — never trust a stored status past its dates (fail-closed).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::plans::{self, FeatureSet, Plan, GRACE_DAYS, PERIOD_DAYS, TIER_FREE, TRIAL_DAYS};

pub const STATUS_TRIALING: &str = "trialing";
pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_PAST_DUE: &str = "past_due";
pub const STATUS_CANCELED: &str = "canceled";
pub const STATUS_UNPAID: &str = "unpaid";
pub const STATUS_EXPIRED: &str = "expired";
pub const STATUS_INCOMPLETE: &str = "incomplete";

pub const PROVIDER_MANUAL: &str = "manual";

/// Absolute seat sanity cap (DoS/typo guard; team "unlimited" still applies
/// below this — 100k seats is an explicit sales conversation, not a form).
pub const ABSOLUTE_SEAT_CAP: i64 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    pub id: String,
    pub org_id: String,
    /// Caller identity (auth subject) that created the subscription. The
    /// owner is the ONLY caller that may read, mutate, or spend this org's
    /// entitlement — org_id alone never confers access (paywall bypass
    /// prevention). Server-side only: never serialized (caller identities
    /// can be tokens or stable user ids — neither belongs in API bodies).
    #[serde(default, skip_serializing)]
    pub owner: String,
    pub tier: String,
    pub status: String,
    pub seats: i64,
    pub cycle: String,
    pub price_cents: i64,
    pub period_start: i64,
    pub period_end: i64,
    pub trial_end: Option<i64>,
    pub cancel_at_period_end: bool,
    pub scheduled_tier: Option<String>,
    pub provider: String,
    pub provider_subscription_id: Option<String>,
    /// Reserved per ADR-0006: inert tier snapshot label. Enforcement reads
    /// `resolve_entitlement`, never this string.
    pub entitlement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entitlement {
    pub tier: String,
    pub status: String,
    pub valid: bool,
    pub features: FeatureSet,
    pub max_seats: i64,
    pub rate_multiplier: i64,
    pub retention_days: i64,
    pub grace_seconds_left: i64,
    pub upgrade_hint: Option<String>,
    pub subscription: Option<Subscription>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubError {
    BadTier,
    BadCycle,
    BadSeats,
    UnreasonableSeats,
    BadOrg,
    OverSeatCap { max: i64 },
    UnknownSubscription,
    AlreadySubscribed,
    CanceledTerminal,
}

impl std::fmt::Display for SubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadTier => write!(f, "unknown tier (want pro, max, or team)"),
            Self::BadCycle => write!(f, "unknown cycle (want monthly or annual)"),
            Self::BadSeats => write!(f, "seats must be >= 1"),
            Self::UnreasonableSeats => {
                write!(f, "seat count unreasonable (max {ABSOLUTE_SEAT_CAP})")
            }
            Self::BadOrg => write!(f, "org_id invalid"),
            Self::OverSeatCap { max } => {
                if *max < 0 {
                    write!(f, "seats must be >= 1")
                } else {
                    write!(f, "seats exceed plan cap of {max}")
                }
            }
            // Generic on purpose: owner mismatch and missing id are
            // INDISTINGUISHABLE (no existence oracle for other orgs).
            Self::UnknownSubscription => write!(f, "subscription not found"),
            Self::AlreadySubscribed => write!(
                f,
                "org already has a live subscription; change or cancel it first"
            ),
            Self::CanceledTerminal => {
                write!(f, "subscription ended; create a new one or reactivate")
            }
        }
    }
}
impl std::error::Error for SubError {}

static SUB_STORE: OnceLock<Mutex<HashMap<String, Subscription>>> = OnceLock::new();
// org_id -> subscription id (one active subscription per org for MVP).
static ORG_INDEX: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn sub_store() -> &'static Mutex<HashMap<String, Subscription>> {
    SUB_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn org_index() -> &'static Mutex<HashMap<String, String>> {
    ORG_INDEX.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_subs() -> std::sync::MutexGuard<'static, HashMap<String, Subscription>> {
    match sub_store().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("subscription store mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

fn lock_index() -> std::sync::MutexGuard<'static, HashMap<String, String>> {
    match org_index().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("subscription index mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

/// Normalize org identity: trim ASCII whitespace. Callers still enforce the
/// length cap; the library rejects empties (fail-closed on blank orgs).
fn normalize_org(org_id: &str) -> Result<String, SubError> {
    let norm = org_id.trim().to_string();
    if norm.is_empty() {
        return Err(SubError::BadOrg);
    }
    Ok(norm)
}

fn check_seats(plan: &Plan, seats: i64) -> Result<(), SubError> {
    if seats < 1 {
        return Err(SubError::BadSeats);
    }
    if seats > ABSOLUTE_SEAT_CAP {
        return Err(SubError::UnreasonableSeats);
    }
    if plan.max_seats >= 0 && seats > plan.max_seats {
        return Err(SubError::OverSeatCap {
            max: plan.max_seats,
        });
    }
    Ok(())
}

/// True while the subscription confers any access (trial active, active,
/// past-due inside grace, or canceled-but-paid-through-period-end).
fn is_live(sub: &Subscription, now: i64) -> bool {
    match sub.status.as_str() {
        STATUS_TRIALING => sub.trial_end.map_or(true, |t| now <= t),
        STATUS_ACTIVE => true,
        STATUS_PAST_DUE => {
            now <= sub
                .period_end
                .saturating_add(GRACE_DAYS.saturating_mul(24 * 3600))
        }
        STATUS_CANCELED => now < sub.period_end,
        _ => false,
    }
}

/// Owner gate: non-owners get the SAME result as a missing subscription
/// (no oracle, no bypass). All mutation entry points must call this.
fn owned(
    store: &HashMap<String, Subscription>,
    id: &str,
    caller: &str,
) -> Result<Subscription, SubError> {
    match store.get(id) {
        Some(sub) if sub.owner == caller => Ok(sub.clone()),
        _ => Err(SubError::UnknownSubscription),
    }
}

fn owned_mut<'a>(
    store: &'a mut HashMap<String, Subscription>,
    id: &str,
    caller: &str,
) -> Result<&'a mut Subscription, SubError> {
    match store.get_mut(id) {
        Some(sub) if sub.owner == caller => Ok(sub),
        _ => Err(SubError::UnknownSubscription),
    }
}

fn price_for(plan: &Plan, cycle: &str, seats: i64) -> Result<i64, SubError> {
    plans::rate_for(plan, cycle)
        .ok_or(SubError::BadCycle)
        .map(|rate| rate.saturating_mul(seats))
}

/// Create: starts `trialing` (14d, no card). Binds the subscription to
/// `caller` (owner). Rejects when the org already holds a LIVE subscription
/// (change/cancel it instead — no silent double-billing, no orphan rows);
/// replaces only dead rows.
pub fn create_subscription(
    org_id: &str,
    tier: &str,
    cycle: &str,
    seats: i64,
    caller: &str,
) -> Result<Subscription, SubError> {
    let org_id = normalize_org(org_id)?;
    if caller.trim().is_empty() {
        return Err(SubError::UnknownSubscription);
    }
    if !plans::is_paid_tier(tier) {
        return Err(SubError::BadTier);
    }
    let plan = plans::plan_for(tier).ok_or(SubError::BadTier)?;
    check_seats(&plan, seats)?;
    let price_cents = price_for(&plan, cycle, seats)?;
    let now = chrono::Utc::now().timestamp();
    // Live-subscription guard + dead-row pruning under one lock pair.
    {
        let subs = lock_subs();
        let index = lock_index();
        if let Some(id) = index.get(&org_id) {
            if let Some(existing) = subs.get(id) {
                if is_live(existing, now) {
                    return Err(SubError::AlreadySubscribed);
                }
            }
        }
    }
    let sub = Subscription {
        id: format!("sub_{}", uuid::Uuid::new_v4()),
        org_id: org_id.clone(),
        owner: caller.to_string(),
        tier: tier.to_string(),
        status: STATUS_TRIALING.to_string(),
        seats,
        cycle: cycle.to_string(),
        price_cents,
        period_start: now,
        period_end: now.saturating_add(PERIOD_DAYS.saturating_mul(24 * 3600)),
        trial_end: Some(now.saturating_add(TRIAL_DAYS.saturating_mul(24 * 3600))),
        cancel_at_period_end: false,
        scheduled_tier: None,
        provider: PROVIDER_MANUAL.to_string(),
        provider_subscription_id: None,
        entitlement: Some(tier.to_string()),
    };
    {
        let mut subs = lock_subs();
        let mut index = lock_index();
        // Prune the previous dead row for this org, if any (bounded memory).
        if let Some(old_id) = index.get(&org_id).cloned() {
            subs.remove(&old_id);
        }
        index.insert(org_id, sub.id.clone());
        subs.insert(sub.id.clone(), sub.clone());
    }
    Ok(sub)
}

/// Direct lookup by id, owner-gated (tests + future admin routes).
#[allow(dead_code)]
pub fn get_subscription(id: &str, caller: &str) -> Option<Subscription> {
    lock_subs().get(id).cloned().filter(|s| s.owner == caller)
}

/// Owner-gated org lookup: non-owners see exactly what a missing org shows.
pub fn get_by_org(org_id: &str, caller: &str) -> Option<Subscription> {
    let org_id = normalize_org(org_id).ok()?;
    let id = lock_index().get(&org_id).cloned()?;
    owned(&lock_subs(), &id, caller).ok()
}

/// Mutable rows must additionally be non-terminal, unless the operation is an
/// explicit revival (`activate`). Lapsed cancels cannot be edited back to
/// life — activate (fresh period) or create anew.
fn ensure_mutable(sub: &Subscription, now: i64) -> Result<(), SubError> {
    match sub.status.as_str() {
        STATUS_EXPIRED | STATUS_INCOMPLETE => Err(SubError::CanceledTerminal),
        STATUS_CANCELED if now >= sub.period_end => Err(SubError::CanceledTerminal),
        _ => Ok(()),
    }
}

/// Activate: record out-of-band (manual) payment. trialing/past_due/unpaid/
/// canceled → active with a fresh period. Terminal `expired`/`incomplete`
/// must create a new subscription (clean history, no resurrection bugs).
pub fn activate_subscription(id: &str, caller: &str) -> Result<Subscription, SubError> {
    let mut store = lock_subs();
    let sub = owned_mut(&mut store, id, caller)?;
    match sub.status.as_str() {
        STATUS_EXPIRED | STATUS_INCOMPLETE => return Err(SubError::CanceledTerminal),
        _ => {}
    }
    let plan = plans::plan_for(&sub.tier).ok_or(SubError::BadTier)?;
    let now = chrono::Utc::now().timestamp();
    sub.status = STATUS_ACTIVE.to_string();
    sub.trial_end = None;
    sub.cancel_at_period_end = false;
    sub.scheduled_tier = None;
    sub.period_start = now;
    sub.period_end = now.saturating_add(PERIOD_DAYS.saturating_mul(24 * 3600));
    sub.price_cents = price_for(&plan, &sub.cycle, sub.seats)?;
    sub.entitlement = Some(sub.tier.clone());
    Ok(sub.clone())
}

/// Change plan/seats/cycle, validated ATOMICALLY before anything is applied:
/// combined upgrade+seats and downgrade+reduce both work in one call.
/// Upgrades apply immediately; downgrades schedule at period end (they paid
/// for it). Returns (subscription, scheduled?).
pub fn change_subscription(
    id: &str,
    caller: &str,
    tier: Option<&str>,
    seats: Option<i64>,
    cycle: Option<&str>,
) -> Result<(Subscription, bool), SubError> {
    // 1. Validate every input before touching state.
    let target_plan = match tier {
        Some(t) => Some(plans::plan_for(t).ok_or(SubError::BadTier)?),
        None => None,
    };
    if let Some(c) = cycle {
        if c != "monthly" && c != "annual" {
            return Err(SubError::BadCycle);
        }
    }
    if let Some(s) = seats {
        if s < 1 {
            return Err(SubError::BadSeats);
        }
        if s > ABSOLUTE_SEAT_CAP {
            return Err(SubError::UnreasonableSeats);
        }
    }

    let mut store = lock_subs();
    let sub = owned_mut(&mut store, id, caller)?;
    let now = chrono::Utc::now().timestamp();
    ensure_mutable(sub, now)?;
    // Note: only an upgrade revokes a pending cancel_at_period_end below
    // (renewed commitment wins); other changes leave it in place.

    // 2. Seat caps against every tier that will govern the seats:
    //    - NOW: the target tier on upgrade (immediate), else current;
    //    - AT PERIOD END: the downgrade target, else a pre-existing schedule.
    //    Seats must fit both (a seats-only bump under a pending downgrade is
    //    rejected — they would breach the cap at period end).
    let current_rank = plans::rank(&sub.tier);
    let target_rank = target_plan
        .as_ref()
        .map(|p| plans::rank(&p.tier))
        .unwrap_or(current_rank);
    let eff_seats = seats.unwrap_or(sub.seats);
    // An explicit tier selection at-or-above the current tier renews
    // commitment and clears a pending scheduled downgrade — otherwise a
    // top-tier org could never grow seats past a stale schedule (deadlock:
    // no higher tier exists to upgrade into). Decided here, APPLIED in
    // step 3 (no partial states on validation failure). Seats-only changes
    // keep the schedule and must fit it.
    let clear_schedule = tier.is_some() && target_rank >= current_rank;
    let current_plan = plans::plan_for(&sub.tier).ok_or(SubError::BadTier)?;
    let now_plan: &Plan = if target_rank > current_rank {
        target_plan.as_ref().unwrap_or(&current_plan)
    } else {
        &current_plan
    };
    check_seats(now_plan, eff_seats)?;
    // End-of-period tier: an explicit downgrade target replaces any schedule;
    // otherwise a pre-existing schedule (unless just cleared) governs.
    let end_tier_name: Option<&str> = if target_rank < current_rank {
        target_plan.as_ref().map(|p| p.tier.as_str())
    } else if clear_schedule {
        None
    } else {
        sub.scheduled_tier.as_deref()
    };
    if let Some(end_tier) = end_tier_name {
        if end_tier != now_plan.tier {
            let end_plan = plans::plan_for(end_tier).ok_or(SubError::BadTier)?;
            // Explicit downgrade the seats cannot fit: reject atomically.
            // (Seats-only bumps over a schedule reject here too.)
            check_seats(&end_plan, eff_seats)?;
        }
    }

    // 3. Apply (all validation already passed — no partial states).
    let mut scheduled = false;
    if clear_schedule {
        sub.scheduled_tier = None;
    }
    if let Some(plan) = target_plan {
        if target_rank > current_rank {
            // Upgrade: immediate, clears any pending downgrade/cancel.
            sub.tier = plan.tier.clone();
            sub.scheduled_tier = None;
            sub.cancel_at_period_end = false;
            sub.entitlement = Some(sub.tier.clone());
        } else if target_rank < current_rank {
            // Downgrade: scheduled at period end; keep paid tier until then.
            if now >= sub.period_end {
                // Period already lapsed: apply now (fail-safe, no free ride).
                sub.tier = plan.tier.clone();
                sub.entitlement = Some(sub.tier.clone());
            } else {
                sub.scheduled_tier = Some(plan.tier.clone());
                scheduled = true;
            }
        }
    }
    if let Some(new_cycle) = cycle {
        sub.cycle = new_cycle.to_string();
    }
    sub.seats = eff_seats;
    let price_plan = plans::plan_for(&sub.tier).ok_or(SubError::BadTier)?;
    sub.price_cents = price_for(&price_plan, &sub.cycle, sub.seats)?;
    Ok((sub.clone(), scheduled))
}

/// Cancel. Default + recommended: at period end (keep paid time).
/// Immediate: ends access now (period_end = now).
pub fn cancel_subscription(
    id: &str,
    caller: &str,
    at_period_end: bool,
) -> Result<Subscription, SubError> {
    let mut store = lock_subs();
    let sub = owned_mut(&mut store, id, caller)?;
    let now = chrono::Utc::now().timestamp();
    ensure_mutable(sub, now)?;
    if at_period_end {
        sub.cancel_at_period_end = true;
    } else {
        sub.status = STATUS_CANCELED.to_string();
        sub.cancel_at_period_end = false;
        sub.period_end = now;
    }
    Ok(sub.clone())
}

/// Set seat count (cap enforced against strictest applicable tier).
pub fn set_seats(id: &str, caller: &str, seats: i64) -> Result<Subscription, SubError> {
    change_subscription(id, caller, None, Some(seats), None).map(|(s, _)| s)
}

/// Resolve the effective entitlement for an org AND caller: stored status is
/// ALWAYS re-evaluated against the clock (lapsed trials/periods fail
/// closed), and a caller that does not own the org's subscription resolves
/// to free — indistinguishable from having none (no oracle, no bypass).
pub fn resolve_entitlement(org_id: &str, caller: &str, feature: Option<&str>) -> Entitlement {
    let now = chrono::Utc::now().timestamp();
    let sub = get_by_org(org_id, caller);
    let Some(sub) = sub else {
        return free_entitlement(feature);
    };
    // Lapsed trial → expired (displayed status, stored row untouched).
    let effective_status: &str =
        if sub.status == STATUS_TRIALING && sub.trial_end.is_some_and(|t| now > t) {
            STATUS_EXPIRED
        } else {
            &sub.status
        };

    let grace_secs = GRACE_DAYS.saturating_mul(24 * 3600);
    let (valid, grace_left) = match effective_status {
        STATUS_TRIALING | STATUS_ACTIVE => (true, 0),
        STATUS_PAST_DUE => {
            let left = sub
                .period_end
                .saturating_add(grace_secs)
                .saturating_sub(now);
            (left > 0, left.max(0))
        }
        STATUS_CANCELED => (now < sub.period_end, 0),
        STATUS_UNPAID | STATUS_EXPIRED | STATUS_INCOMPLETE => (false, 0),
        _ => (false, 0),
    };

    let (tier, features, max_seats, rate_multiplier, retention_days) = if valid {
        match plans::plan_for(&sub.tier) {
            Some(p) => (
                sub.tier.clone(),
                p.features,
                p.max_seats,
                p.rate_multiplier,
                p.retention_days,
            ),
            None => (TIER_FREE.to_string(), FeatureSet::free(), 0, 1, 0),
        }
    } else {
        (TIER_FREE.to_string(), FeatureSet::free(), 0, 1, 0)
    };

    let upgrade_hint = match feature {
        Some(f) if valid && features.get(f) == Some(false) => {
            FeatureSet::min_tier_for(f).map(|t| format!("{f} requires {t} or higher"))
        }
        Some(f) if !valid => FeatureSet::min_tier_for(f)
            .map(|t| format!("subscription not active; {f} requires {t} or higher")),
        Some(_) => None,
        None if !valid => Some("subscription not active; upgrade to Pro or higher".to_string()),
        None => None,
    };

    Entitlement {
        tier,
        status: effective_status.to_string(),
        valid,
        features,
        max_seats,
        rate_multiplier,
        retention_days,
        grace_seconds_left: grace_left,
        upgrade_hint,
        subscription: Some(sub),
    }
}

fn free_entitlement(feature: Option<&str>) -> Entitlement {
    let upgrade_hint = match feature {
        Some(f) => FeatureSet::min_tier_for(f).map(|t| format!("{f} requires {t} or higher")),
        None => None,
    };
    Entitlement {
        tier: TIER_FREE.to_string(),
        status: "none".to_string(),
        valid: false,
        features: FeatureSet::free(),
        max_seats: 0,
        rate_multiplier: 1,
        retention_days: 0,
        grace_seconds_left: 0,
        upgrade_hint,
        subscription: None,
    }
}

/// Clear store (tests).
#[allow(dead_code)]
pub fn clear_subscriptions() {
    lock_subs().clear();
    lock_index().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;

    const ME: &str = "owner-1";
    const YOU: &str = "owner-2";

    fn setup() -> Subscription {
        clear_subscriptions();
        create_subscription("org-1", "pro", "monthly", 3, ME).expect("create ok")
    }

    #[test]
    fn create_trial_prices_per_seat() {
        let _guard = test_sync::lock();
        let sub = setup();
        assert_eq!(sub.status, STATUS_TRIALING);
        assert_eq!(sub.price_cents, 1200 * 3);
        assert!(sub.trial_end.unwrap() > chrono::Utc::now().timestamp());
        assert_eq!(sub.provider, PROVIDER_MANUAL);
        // Annual reprices.
        let annual = create_subscription("org-a", "max", "annual", 2, ME).unwrap();
        assert_eq!(annual.price_cents, 2400 * 2);
    }

    #[test]
    fn create_rejects_bad_input() {
        let _guard = test_sync::lock();
        clear_subscriptions();
        assert_eq!(
            create_subscription("o", "ultra", "monthly", 1, ME).unwrap_err(),
            SubError::BadTier
        );
        assert_eq!(
            create_subscription("o", "pro", "weekly", 1, ME).unwrap_err(),
            SubError::BadCycle
        );
        assert_eq!(
            create_subscription("o", "pro", "monthly", 0, ME).unwrap_err(),
            SubError::BadSeats
        );
        assert_eq!(
            create_subscription("o", "pro", "monthly", 26, ME).unwrap_err(),
            SubError::OverSeatCap { max: 25 }
        );
        // Team unlimited (within the absolute sanity cap).
        assert!(create_subscription("o", "team", "monthly", 5000, ME).is_ok());
        assert_eq!(
            create_subscription("o2", "team", "monthly", ABSOLUTE_SEAT_CAP + 1, ME).unwrap_err(),
            SubError::UnreasonableSeats
        );
        assert_eq!(
            create_subscription("", "pro", "monthly", 1, ME).unwrap_err(),
            SubError::BadOrg
        );
        assert_eq!(
            create_subscription("   ", "pro", "monthly", 1, ME).unwrap_err(),
            SubError::BadOrg
        );
    }

    #[test]
    fn upgrade_immediate_downgrade_scheduled() {
        let _guard = test_sync::lock();
        let sub = setup();
        // Upgrade pro -> team: immediate.
        let (up, scheduled) = change_subscription(&sub.id, ME, Some("team"), None, None).unwrap();
        assert!(!scheduled);
        assert_eq!(up.tier, "team");
        assert_eq!(up.price_cents, 4900 * 3);
        // Downgrade team -> pro: scheduled, current kept.
        let (down, scheduled) = change_subscription(&sub.id, ME, Some("pro"), None, None).unwrap();
        assert!(scheduled);
        assert_eq!(down.tier, "team");
        assert_eq!(down.scheduled_tier.as_deref(), Some("pro"));
        // Seats capped against scheduled (stricter) tier.
        assert_eq!(
            change_subscription(&sub.id, ME, None, Some(26), None).unwrap_err(),
            SubError::OverSeatCap { max: 25 }
        );
        // Combined upgrade + seats in one atomic call. The seats bump also
        // clears the stale scheduled downgrade (renewed commitment).
        let (up2, scheduled) =
            change_subscription(&sub.id, ME, Some("team"), Some(5000), None).unwrap();
        assert!(!scheduled);
        assert_eq!(up2.tier, "team");
        assert_eq!(up2.seats, 5000);
        assert_eq!(up2.price_cents, 4900 * 5000);
        assert_eq!(up2.scheduled_tier, None);
        // Combined downgrade + reduce in one atomic call.
        let (down2, scheduled) =
            change_subscription(&sub.id, ME, Some("pro"), Some(10), None).unwrap();
        assert!(scheduled);
        assert_eq!(down2.tier, "team"); // kept until period end
        assert_eq!(down2.seats, 10);
        // Downgrade WITHOUT reducing over-cap seats is rejected atomically
        // (nothing applied).
        let err = change_subscription(&sub.id, ME, Some("max"), Some(500), None).unwrap_err();
        assert_eq!(err, SubError::OverSeatCap { max: 100 });
        let after = get_subscription(&sub.id, ME).unwrap();
        assert_eq!((after.tier.as_str(), after.seats), ("team", 10));
    }

    #[test]
    fn cancel_period_end_keeps_access_then_lapses() {
        let _guard = test_sync::lock();
        let sub = setup();
        let c = cancel_subscription(&sub.id, ME, true).unwrap();
        assert!(c.cancel_at_period_end);
        assert_eq!(c.status, STATUS_TRIALING);
        // Still entitled during paid/trial time.
        assert!(resolve_entitlement("org-1", ME, None).valid);
        // Immediate cancel ends now.
        let c2 = cancel_subscription(&sub.id, ME, false).unwrap();
        assert_eq!(c2.status, STATUS_CANCELED);
        assert!(!resolve_entitlement("org-1", ME, None).valid);
        // Lapsed immediate-cancel is terminal for edits (activate or recreate).
        assert_eq!(
            change_subscription(&sub.id, ME, None, Some(2), None).unwrap_err(),
            SubError::CanceledTerminal
        );
        // ... but activate revives with a fresh period.
        let revived = activate_subscription(&sub.id, ME).unwrap();
        assert_eq!(revived.status, STATUS_ACTIVE);
        assert!(resolve_entitlement("org-1", ME, None).valid);
    }

    #[test]
    fn trial_lapse_resolves_expired_fail_closed() {
        let _guard = test_sync::lock();
        clear_subscriptions();
        let mut sub = create_subscription("org-old", "pro", "monthly", 1, ME).unwrap();
        // Backdate trial + period (simulates 30 days passing; store untouched).
        {
            let mut store = lock_subs();
            let row = store.get_mut(&sub.id).unwrap();
            let past = chrono::Utc::now().timestamp() - 30 * 24 * 3600;
            row.period_start = past - 30 * 24 * 3600;
            row.period_end = past;
            row.trial_end = Some(past);
            sub = row.clone();
        }
        let ent = resolve_entitlement("org-old", ME, Some("dry_run"));
        assert_eq!(ent.status, STATUS_EXPIRED);
        assert!(!ent.valid);
        assert!(!ent.features.dry_run);
        assert!(ent.upgrade_hint.unwrap().contains("pro"));
        // Stored row still says trialing (lazy resolution, no silent writes).
        assert_eq!(
            get_subscription(&sub.id, ME).unwrap().status,
            STATUS_TRIALING
        );
    }

    #[test]
    fn activate_starts_fresh_period() {
        let _guard = test_sync::lock();
        let sub = setup();
        let a = activate_subscription(&sub.id, ME).unwrap();
        assert_eq!(a.status, STATUS_ACTIVE);
        assert_eq!(a.trial_end, None);
        assert!(resolve_entitlement("org-1", ME, Some("dry_run")).valid);
        // Terminal states cannot resurrect.
        clear_subscriptions();
        let mut s2 = create_subscription("org-x", "pro", "monthly", 1, ME).unwrap();
        {
            let mut store = lock_subs();
            store.get_mut(&s2.id).unwrap().status = STATUS_EXPIRED.to_string();
            s2.status = STATUS_EXPIRED.to_string();
        }
        assert_eq!(
            activate_subscription(&s2.id, ME).unwrap_err(),
            SubError::CanceledTerminal
        );
    }

    #[test]
    fn past_due_grace_then_unpaid() {
        let _guard = test_sync::lock();
        clear_subscriptions();
        let sub = create_subscription("org-pd", "max", "monthly", 2, ME).unwrap();
        let now = chrono::Utc::now().timestamp();
        {
            let mut store = lock_subs();
            let row = store.get_mut(&sub.id).unwrap();
            row.status = STATUS_PAST_DUE.to_string();
            row.period_end = now - 24 * 3600; // 1 day into grace.
        }
        let ent = resolve_entitlement("org-pd", ME, None);
        assert!(ent.valid);
        assert!(ent.grace_seconds_left > 12 * 24 * 3600);
        assert!(ent.grace_seconds_left <= 14 * 24 * 3600);
        // 20 days later: grace exhausted → invalid.
        {
            let mut store = lock_subs();
            store.get_mut(&sub.id).unwrap().period_end = now - 20 * 24 * 3600;
        }
        let ent2 = resolve_entitlement("org-pd", ME, None);
        assert!(!ent2.valid);
        assert_eq!(ent2.grace_seconds_left, 0);
    }

    #[test]
    fn proves_ask_on_unknown_org_and_feature() {
        let _guard = test_sync::lock();
        clear_subscriptions();
        let ent = resolve_entitlement("ghost", ME, Some("dry_run"));
        assert!(!ent.valid);
        assert_eq!(ent.tier, TIER_FREE);
        assert!(ent.upgrade_hint.unwrap().contains("pro"));
        // Unknown feature never allowed, even when entitled.
        let _ = setup();
        let ent2 = resolve_entitlement("org-1", ME, Some("teleport"));
        assert!(ent2.valid); // subscription valid...
        assert!(ent2.upgrade_hint.is_none()); // ...but no hint for unknown features
    }

    #[test]
    fn proves_no_cross_org_claim_or_oracle() {
        let _guard = test_sync::lock();
        // ME owns a TEAM subscription on org-victim.
        clear_subscriptions();
        let sub = create_subscription("org-victim", "team", "monthly", 5, ME).unwrap();
        // YOU (a different caller) sees exactly what a missing org shows:
        // free entitlement, no hint leak beyond the generic upsell.
        let ent_you = resolve_entitlement("org-victim", YOU, Some("siem_export"));
        let ent_ghost = resolve_entitlement("no-such-org", YOU, Some("siem_export"));
        assert_eq!(ent_you, ent_ghost);
        assert!(!ent_you.valid);
        // Mutations are indistinguishable from missing (404, same message).
        assert_eq!(
            change_subscription(&sub.id, YOU, Some("pro"), None, None).unwrap_err(),
            SubError::UnknownSubscription
        );
        assert_eq!(
            cancel_subscription(&sub.id, YOU, true).unwrap_err(),
            SubError::UnknownSubscription
        );
        assert_eq!(
            activate_subscription(&sub.id, YOU).unwrap_err(),
            SubError::UnknownSubscription
        );
        assert_eq!(
            set_seats(&sub.id, YOU, 1).unwrap_err(),
            SubError::UnknownSubscription
        );
        assert!(get_subscription(&sub.id, YOU).is_none());
        assert!(get_by_org("org-victim", YOU).is_none());
        // Owner unaffected.
        assert!(resolve_entitlement("org-victim", ME, Some("siem_export")).valid);
    }

    #[test]
    fn proves_no_double_billing_or_orphans() {
        let _guard = test_sync::lock();
        clear_subscriptions();
        let first = create_subscription("org-dup", "pro", "monthly", 1, ME).unwrap();
        // Second create while live → 409, index still points at the first.
        assert_eq!(
            create_subscription("org-dup", "max", "monthly", 1, ME).unwrap_err(),
            SubError::AlreadySubscribed
        );
        assert_eq!(get_by_org("org-dup", ME).unwrap().id, first.id);
        // After immediate cancel (dead), create replaces and prunes the row.
        cancel_subscription(&first.id, ME, false).unwrap();
        let second = create_subscription("org-dup", "max", "monthly", 1, ME).unwrap();
        assert_ne!(second.id, first.id);
        assert!(get_subscription(&first.id, ME).is_none());
        assert_eq!(get_by_org("org-dup", ME).unwrap().id, second.id);
        // Whitespace variants resolve to the same org (no shadow subs).
        assert_eq!(
            create_subscription("  org-dup  ", "pro", "monthly", 1, ME).unwrap_err(),
            SubError::AlreadySubscribed
        );
        assert!(resolve_entitlement("  org-dup ", ME, None).valid);
    }
}
