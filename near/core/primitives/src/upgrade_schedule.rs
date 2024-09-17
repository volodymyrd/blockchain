use chrono::{DateTime, NaiveDateTime, Utc};
use near_primitives_core::types::ProtocolVersion;
use std::env;

const NEAR_TESTS_PROTOCOL_UPGRADE_OVERRIDE: &str = "NEAR_TESTS_PROTOCOL_UPGRADE_OVERRIDE";

#[derive(thiserror::Error, Clone, Debug)]
pub enum ProtocolUpgradeVotingScheduleError {
    #[error("The final upgrade must be the client protocol version! final version: {0}, client version: {1}")]
    InvalidFinalUpgrade(ProtocolVersion, ProtocolVersion),
    #[error("The upgrades must be sorted by datetime!")]
    InvalidDateTimeOrder,
    #[error("The upgrades must be sorted and increasing by one!")]
    InvalidProtocolVersionOrder,

    #[error("The environment override has an invalid format! Input: {0} Error: {1}")]
    InvalidOverrideFormat(String, String),
}

type ProtocolUpgradeVotingScheduleRaw = Vec<(DateTime<Utc>, ProtocolVersion)>;

/// Defines a schedule for validators to vote for the protocol version upgrades.
/// Multiple protocol version upgrades can be scheduled. The default schedule is
/// empty and in that case the node will always vote for the client protocol
/// version.
#[derive(Clone, Debug, PartialEq)]
pub struct ProtocolUpgradeVotingSchedule {
    // The highest protocol version supported by the client. This class will
    // check that the schedule ends with this version.
    client_protocol_version: ProtocolVersion,

    /// The schedule is a sorted list of (datetime, version) tuples. The node
    /// should vote for the highest version that is less than or equal to the
    /// current time.
    schedule: ProtocolUpgradeVotingScheduleRaw,
}

impl ProtocolUpgradeVotingSchedule {
    /// This method creates an instance of the ProtocolUpgradeVotingSchedule
    /// that will immediately vote for the client protocol version.
    pub fn new_immediate(client_protocol_version: ProtocolVersion) -> Self {
        Self {
            client_protocol_version,
            schedule: vec![],
        }
    }

    /// This method creates an instance of the ProtocolUpgradeVotingSchedule.
    ///
    /// It will first check if the NEAR_TESTS_PROTOCOL_UPGRADE_OVERRIDE is set
    /// in the environment and if so this override will be used as schedule.
    /// This should only be used in tests, in particular in tests that in some
    /// way test neard upgrades.
    ///
    /// Otherwise it will use the provided schedule.
    pub fn new_from_env_or_schedule(
        client_protocol_version: ProtocolVersion,
        mut schedule: ProtocolUpgradeVotingScheduleRaw,
    ) -> Result<Self, ProtocolUpgradeVotingScheduleError> {
        let env_override = env::var(NEAR_TESTS_PROTOCOL_UPGRADE_OVERRIDE);
        if let Ok(env_override) = env_override {
            schedule = Self::parse_override(&env_override)?;
            tracing::warn!(
                target: "protocol_upgrade",
                ?schedule,
                "Setting protocol upgrade override. This is fine in tests but should be avoided otherwise"
            );
        }

        // Sanity and invariant checks.

        // The final upgrade must be the client protocol version.
        if let Some((_, version)) = schedule.last() {
            if *version != client_protocol_version {
                return Err(ProtocolUpgradeVotingScheduleError::InvalidFinalUpgrade(
                    *version,
                    client_protocol_version,
                ));
            }
        }

        // The upgrades must be sorted by datetime.
        for i in 1..schedule.len() {
            let prev_time = schedule[i - 1].0;
            let this_time = schedule[i].0;
            if !(prev_time < this_time) {
                return Err(ProtocolUpgradeVotingScheduleError::InvalidDateTimeOrder);
            }
        }

        // The upgrades must be increasing by 1.
        for i in 1..schedule.len() {
            let prev_protocol_version = schedule[i - 1].1;
            let this_protocol_version = schedule[i].1;
            if prev_protocol_version + 1 != this_protocol_version {
                return Err(ProtocolUpgradeVotingScheduleError::InvalidProtocolVersionOrder);
            }
        }

        tracing::debug!(target: "protocol_upgrade", ?schedule, "created protocol upgrade schedule");

        Ok(Self {
            client_protocol_version,
            schedule,
        })
    }

    /// This method returns the protocol version that the node should vote for.
    #[cfg(feature = "clock")]
    pub(crate) fn get_protocol_version(
        &self,
        now: DateTime<Utc>,
        // Protocol version that will be used in the next epoch.
        next_epoch_protocol_version: ProtocolVersion,
    ) -> ProtocolVersion {
        if next_epoch_protocol_version >= self.client_protocol_version {
            return self.client_protocol_version;
        }

        if self.schedule.is_empty() {
            return self.client_protocol_version;
        }

        // The datetime values in the schedule are sorted in ascending order.
        // Find the first datetime value that is less than the current time
        // and higher than next_epoch_protocol_version.
        // The schedule is sorted and the last value is the client_protocol_version
        // so we are guaranteed to find a correct protocol version.
        let mut result = next_epoch_protocol_version;
        for (time, version) in &self.schedule {
            if now < *time {
                break;
            }
            result = *version;
            if *version > next_epoch_protocol_version {
                break;
            }
        }

        result
    }

    /// Returns the schedule. Should only be used for exporting metrics.
    pub fn schedule(&self) -> &Vec<(DateTime<Utc>, ProtocolVersion)> {
        &self.schedule
    }

    /// A helper method to parse the datetime string.
    pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        let datetime = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")?;
        let datetime = DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc);
        Ok(datetime)
    }

    // Parse the protocol version override from the environment.
    // The format is comma separate datetime:=version pairs.
    fn parse_override(
        override_str: &str,
    ) -> Result<ProtocolUpgradeVotingScheduleRaw, ProtocolUpgradeVotingScheduleError> {
        // The special value "now" means that the upgrade should happen immediately.
        if override_str.to_lowercase() == "now" {
            return Ok(vec![]);
        }

        let mut result = vec![];
        let datetime_and_version_vec = override_str.split(',').collect::<Vec<_>>();
        for datetime_and_version in datetime_and_version_vec {
            let datetime_and_version = datetime_and_version.split('=').collect::<Vec<_>>();
            let [datetime, version] = datetime_and_version[..] else {
                let input = format!("{:?}", datetime_and_version);
                let error = "The override must be in the format datetime=version!".to_string();
                return Err(ProtocolUpgradeVotingScheduleError::InvalidOverrideFormat(
                    input, error,
                ));
            };

            let datetime = Self::parse_datetime(datetime).map_err(|err| {
                ProtocolUpgradeVotingScheduleError::InvalidOverrideFormat(
                    datetime.to_string(),
                    err.to_string(),
                )
            })?;
            let version = version.parse::<u32>().map_err(|err| {
                ProtocolUpgradeVotingScheduleError::InvalidOverrideFormat(
                    version.to_string(),
                    err.to_string(),
                )
            })?;
            result.push((datetime, version));
        }
        Ok(result)
    }
}
