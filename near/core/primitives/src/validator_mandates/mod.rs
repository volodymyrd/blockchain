use crate::types::{validator_stake::ValidatorStake, ValidatorId};
use borsh::{BorshDeserialize, BorshSerialize};
use near_primitives_core::types::Balance;
use near_schema_checker_lib::ProtocolSchema;

mod compute_price;

/// Represents the configuration of [`ValidatorMandates`]. Its parameters are expected to remain
/// valid for one epoch.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    ProtocolSchema,
)]
pub struct ValidatorMandatesConfig {
    /// The desired number of mandates required per shard.
    target_mandates_per_shard: usize,
    /// The number of shards for the referenced epoch.
    num_shards: usize,
}

impl ValidatorMandatesConfig {
    /// Constructs a new configuration.
    ///
    /// # Panics
    ///
    /// Panics in the following cases:
    ///
    /// - If `stake_per_mandate` is 0 as this would lead to division by 0.
    /// - If `num_shards` is zero.
    pub fn new(target_mandates_per_shard: usize, num_shards: usize) -> Self {
        assert!(num_shards > 0, "there should be at least one shard");
        Self { target_mandates_per_shard, num_shards }
    }
}

/// The mandates for a set of validators given a [`ValidatorMandatesConfig`].
///
/// A mandate is a liability for a validator to validate a shard. Depending on its stake and the
/// `stake_per_mandate` specified in `ValidatorMandatesConfig`, a validator may hold multiple
/// mandates. Each mandate may be assigned to a different shard. The assignment of mandates to
/// shards is calculated with [`Self::sample`], typically at every height.
///
/// See #9983 for context and links to resources that introduce mandates.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    ProtocolSchema,
)]
pub struct ValidatorMandates {
    /// The configuration applied to the mandates.
    config: ValidatorMandatesConfig,
    /// The amount of stake a whole mandate is worth.
    stake_per_mandate: Balance,
    /// Each element represents a validator mandate held by the validator with the given id.
    ///
    /// The id of a validator who holds `n >= 0` mandates occurs `n` times in the vector.
    mandates: Vec<ValidatorId>,
    /// Each element represents a partial validator mandate held by the validator with the given id.
    /// For example, an element `(1, 42)` represents the partial mandate of the validator with id 1
    /// which has a weight of 42.
    ///
    /// Validators whose stake can be distributed across mandates without remainder are not
    /// represented in this vector.
    partials: Vec<(ValidatorId, Balance)>,
}

impl ValidatorMandates {
    /// Initiates mandates corresponding to the provided `validators`. The validators must be sorted
    /// by id in ascending order, so the validator with `ValidatorId` equal to `i` is given by
    /// `validators[i]`.
    ///
    /// Only full mandates are assigned, partial mandates are dropped. For example, when the stake
    /// required for a mandate is 5 and a validator has staked 12, then it will obtain 2 mandates.
    pub fn new(config: ValidatorMandatesConfig, validators: &[ValidatorStake]) -> Self {
        let stakes: Vec<Balance> = validators.iter().map(|v| v.stake()).collect();
        let stake_per_mandate = compute_price::compute_mandate_price(config, &stakes);
        let num_mandates_per_validator: Vec<u16> =
            validators.iter().map(|v| v.num_mandates(stake_per_mandate)).collect();
        let num_total_mandates =
            num_mandates_per_validator.iter().map(|&num| usize::from(num)).sum();
        let mut mandates: Vec<ValidatorId> = Vec::with_capacity(num_total_mandates);

        for i in 0..validators.len() {
            for _ in 0..num_mandates_per_validator[i] {
                // Each validator's position corresponds to its id.
                mandates.push(i as ValidatorId);
            }
        }

        // Not counting partials towards `required_mandates` as the weight of partials and its
        // distribution across shards may vary widely.
        //
        // Construct vector with capacity as most likely some validators' stake will not be evenly
        // divided by `config.stake_per_mandate`, i.e. some validators will have partials.
        let mut partials = Vec::with_capacity(validators.len());
        for i in 0..validators.len() {
            let partial_weight = validators[i].partial_mandate_weight(stake_per_mandate);
            if partial_weight > 0 {
                partials.push((i as ValidatorId, partial_weight));
            }
        }

        Self { config, stake_per_mandate, mandates, partials }
    }
}

#[cfg(feature = "rand")]
mod validator_mandates_sample {
    use super::*;
    use itertools::Itertools;
    use rand::{seq::SliceRandom, Rng};

    impl ValidatorMandates {
        /// Returns a validator assignment obtained by shuffling mandates and assigning them to shards.
        /// Shard ids are shuffled as well in this process to avoid a bias lower shard ids, see
        /// [`ShuffledShardIds`].
        ///
        /// It clones mandates since [`ValidatorMandates`] is supposed to be valid for an epoch, while a
        /// new assignment is calculated at every height.
        pub fn sample<R>(&self, rng: &mut R) -> ChunkValidatorStakeAssignment
        where
            R: Rng + ?Sized,
        {
            // Shuffling shard ids to avoid a bias towards lower ids, see [`ShuffledShardIds`]. We
            // do two separate shuffes for full and partial mandates to reduce the likelihood of
            // assigning fewer full _and_ partial mandates to the _same_ shard.
            let shard_ids_for_mandates = ShuffledShardIds::new(rng, self.config.num_shards);
            let shard_ids_for_partials = ShuffledShardIds::new(rng, self.config.num_shards);

            let shuffled_mandates = self.shuffled_mandates(rng);
            let shuffled_partials = self.shuffled_partials(rng);

            // Distribute shuffled mandates and partials across shards. For each shard with `shard_id`
            // in `[0, num_shards)`, we take the elements of the vector with index `i` such that `i %
            // num_shards == shard_id`.
            //
            // Assume, for example, there are 10 mandates and 4 shards. Then for `shard_id = 1` we
            // collect the mandates with indices 1, 5, and 9.
            let stake_per_mandate = self.stake_per_mandate;
            let mut stake_assignment_per_shard =
                vec![std::collections::HashMap::new(); self.config.num_shards];
            for shard_id in 0..self.config.num_shards {
                // Achieve shard id shuffling by writing to the position of the alias of `shard_id`.
                let mandates_assignment =
                    &mut stake_assignment_per_shard[shard_ids_for_mandates.get_alias(shard_id)];

                // For the current `shard_id`, collect mandates with index `i` such that
                // `i % num_shards == shard_id`.
                for idx in (shard_id..shuffled_mandates.len()).step_by(self.config.num_shards) {
                    let validator_id = shuffled_mandates[idx];
                    *mandates_assignment.entry(validator_id).or_default() += stake_per_mandate;
                }

                // Achieve shard id shuffling by writing to the position of the alias of `shard_id`.
                let partials_assignment =
                    &mut stake_assignment_per_shard[shard_ids_for_partials.get_alias(shard_id)];

                // For the current `shard_id`, collect partials with index `i` such that
                // `i % num_shards == shard_id`.
                for idx in (shard_id..shuffled_partials.len()).step_by(self.config.num_shards) {
                    let (validator_id, partial_weight) = shuffled_partials[idx];
                    *partials_assignment.entry(validator_id).or_default() += partial_weight;
                }
            }

            // Deterministically shuffle the validator order for each shard
            let mut ordered_stake_assignment_per_shard = Vec::with_capacity(self.config.num_shards);
            for shard_id in 0..self.config.num_shards {
                // first sort the validators by id then shuffle using rng
                let stake_assignment = &stake_assignment_per_shard[shard_id];
                let mut ordered_validator_ids = stake_assignment.keys().sorted().collect_vec();
                ordered_validator_ids.shuffle(rng);
                let ordered_mandate_assignment = ordered_validator_ids
                    .into_iter()
                    .map(|validator_id| (*validator_id, stake_assignment[validator_id]))
                    .collect_vec();
                ordered_stake_assignment_per_shard.push(ordered_mandate_assignment);
            }

            ordered_stake_assignment_per_shard
        }

        /// Clones the contained mandates and shuffles them. Cloning is required as a shuffle happens at
        /// every height while the `ValidatorMandates` are to be valid for an epoch.
        pub(super) fn shuffled_mandates<R>(&self, rng: &mut R) -> Vec<ValidatorId>
        where
            R: Rng + ?Sized,
        {
            let mut shuffled_mandates = self.mandates.clone();
            shuffled_mandates.shuffle(rng);
            shuffled_mandates
        }

        /// Clones the contained partials and shuffles them. Cloning is required as a shuffle happens at
        /// every height while the `ValidatorMandates` are to be valid for an epoch.
        pub(super) fn shuffled_partials<R>(&self, rng: &mut R) -> Vec<(ValidatorId, Balance)>
        where
            R: Rng + ?Sized,
        {
            let mut shuffled_partials = self.partials.clone();
            shuffled_partials.shuffle(rng);
            shuffled_partials
        }
    }
}

/// Represents an assignment of [`ValidatorMandates`] for a specific height.
///
/// Contains one vec per shard, with the position in the vector corresponding to `shard_id` in
/// `0..num_shards`. Each element is a tuple of `ValidatorId`, total stake they have in the
/// corresponding shards. A validator whose id is not in any vec has not been assigned to the shard.
///
/// For example, `mandates_per_shard[0]` gives us the entries of shard with id 0.
/// Elements of `mandates_per_shard[0]` can be [(validator3, stake), (validator7, stake)]
pub type ChunkValidatorStakeAssignment = Vec<Vec<(ValidatorId, Balance)>>;
