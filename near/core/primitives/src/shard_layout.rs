use crate::hash::CryptoHash;
use crate::types::{AccountId, NumShards};
use borsh::{BorshDeserialize, BorshSerialize};
use near_primitives_core::types::ShardId;
use near_schema_checker_lib::ProtocolSchema;
use std::collections::HashMap;
use std::{fmt, str};

/// This file implements two data structure `ShardLayout` and `ShardUId`
///
/// `ShardLayout`
/// A versioned struct that contains all information needed to assign accounts
/// to shards. Because of re-sharding, the chain may use different shard layout to
/// split shards at different times.
/// Currently, `ShardLayout` is stored as part of `EpochConfig`, which is generated each epoch
/// given the epoch protocol version.
/// In mainnet/testnet, we use two shard layouts since re-sharding has only happened once.
/// It is stored as part of genesis config, see default_simple_nightshade_shard_layout()
/// Below is an overview for some important functionalities of ShardLayout interface.
///
/// `version`
/// `ShardLayout` has a version number. The version number should increment as when sharding changes.
/// This guarantees the version number is unique across different shard layouts, which in turn guarantees
/// `ShardUId` is different across shards from different shard layouts, as `ShardUId` includes
/// `version` and `shard_id`
///
/// `get_parent_shard_id` and `get_split_shard_ids`
/// `ShardLayout` also includes information needed for resharding. In particular, it encodes
/// which shards from the previous shard layout split to which shards in the following shard layout.
/// If shard A in shard layout 0 splits to shard B and C in shard layout 1,
/// we call shard A the parent shard of shard B and C.
/// Note that a shard can only have one parent shard. For example, the following case will be prohibited,
/// a shard C in shard layout 1 contains accounts in both shard A and B in shard layout 0.
/// Parent/split shard information can be accessed through these two functions.
///
/// `account_id_to_shard_id`
///  Maps an account to the shard that it belongs to given a shard_layout
///
/// `ShardUId`
/// `ShardUId` is a unique representation for shards from different shard layouts.
/// Comparing to `ShardId`, which is just an ordinal number ranging from 0 to NUM_SHARDS-1,
/// `ShardUId` provides a way to unique identify shards when shard layouts may change across epochs.
/// This is important because we store states indexed by shards in our database, so we need a
/// way to unique identify shard even when shards change across epochs.
/// Another difference between `ShardUId` and `ShardId` is that `ShardUId` should only exist in
/// a node's internal state while `ShardId` can be exposed to outside APIs and used in protocol
/// level information (for example, `ShardChunkHeader` contains `ShardId` instead of `ShardUId`)

pub type ShardVersion = u32;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ShardLayout {
    V1(ShardLayoutV1),
}

/// A map that maps shards from the last shard layout to shards that it splits to in this shard layout.
/// Instead of using map, we just use a vec here because shard_id ranges from 0 to num_shards-1
/// For example, if a shard layout with only shard 0 splits into shards 0, 1, 2, 3, the ShardsSplitMap
/// will be `[[0, 1, 2, 3]]`
type ShardSplitMap = Vec<Vec<ShardId>>;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ShardLayoutV1 {
    /// The boundary accounts are the accounts on boundaries between shards.
    /// Each shard contains a range of accounts from one boundary account to
    /// another - or the smallest or largest account possible. The total
    /// number of shards is equal to the number of boundary accounts plus 1.
    boundary_accounts: Vec<AccountId>,
    /// Maps shards from the last shard layout to shards that it splits to in this shard layout,
    /// Useful for constructing states for the shards.
    /// None for the genesis shard layout
    shards_split_map: Option<ShardSplitMap>,
    /// Maps shard in this shard layout to their parent shard
    /// Since shard_ids always range from 0 to num_shards - 1, we use vec instead of a hashmap
    to_parent_shard_map: Option<Vec<ShardId>>,
    /// Version of the shard layout, this is useful for uniquely identify the shard layout
    version: ShardVersion,
}

#[derive(Debug)]
pub enum ShardLayoutError {
    InvalidShardIdError { shard_id: ShardId },
}

impl ShardLayout {
    /* Some constructors */
    /// Return a V1 Shardlayout
    pub fn v1(
        boundary_accounts: Vec<AccountId>,
        shards_split_map: Option<ShardSplitMap>,
        version: ShardVersion,
    ) -> Self {
        let to_parent_shard_map = if let Some(shards_split_map) = &shards_split_map {
            let mut to_parent_shard_map = HashMap::new();
            let num_shards = (boundary_accounts.len() + 1) as NumShards;
            for (parent_shard_id, shard_ids) in shards_split_map.iter().enumerate() {
                for &shard_id in shard_ids {
                    let prev = to_parent_shard_map.insert(shard_id, parent_shard_id as ShardId);
                    assert!(prev.is_none(), "no shard should appear in the map twice");
                    assert!(shard_id < num_shards, "shard id should be valid");
                }
            }
            Some(
                (0..num_shards)
                    .map(|shard_id| to_parent_shard_map[&shard_id])
                    .collect(),
            )
        } else {
            None
        };
        Self::V1(ShardLayoutV1 {
            boundary_accounts,
            shards_split_map,
            to_parent_shard_map,
            version,
        })
    }

    /// Returns a V1 ShardLayout. It is only used in tests
    pub fn v1_test() -> Self {
        ShardLayout::v1(
            vec!["abc", "foo", "test0"]
                .into_iter()
                .map(|s| s.parse().unwrap())
                .collect(),
            Some(vec![vec![0, 1, 2, 3]]),
            1,
        )
    }

    /// Returns the simple nightshade layout that we use in production
    pub fn get_simple_nightshade_layout() -> ShardLayout {
        ShardLayout::v1(
            vec!["aurora", "aurora-0", "kkuuue2akv_1630967379.near"]
                .into_iter()
                .map(|s| s.parse().unwrap())
                .collect(),
            Some(vec![vec![0, 1, 2, 3]]),
            1,
        )
    }

    /// Returns the simple nightshade layout, version 2, that will be used in production.
    pub fn get_simple_nightshade_layout_v2() -> ShardLayout {
        ShardLayout::v1(
            vec![
                "aurora",
                "aurora-0",
                "kkuuue2akv_1630967379.near",
                "tge-lockup.sweat",
            ]
            .into_iter()
            .map(|s| s.parse().unwrap())
            .collect(),
            Some(vec![vec![0], vec![1], vec![2], vec![3, 4]]),
            2,
        )
    }

    /// Returns the simple nightshade layout, version 3, that will be used in production.
    pub fn get_simple_nightshade_layout_v3() -> ShardLayout {
        ShardLayout::v1(
            vec![
                "aurora",
                "aurora-0",
                "game.hot.tg",
                "kkuuue2akv_1630967379.near",
                "tge-lockup.sweat",
            ]
            .into_iter()
            .map(|s| s.parse().unwrap())
            .collect(),
            Some(vec![vec![0], vec![1], vec![2, 3], vec![4], vec![5]]),
            3,
        )
    }

    /// This layout is used only in resharding tests. It allows testing of any features which were
    /// introduced after the last layout upgrade in production. Currently it is built on top of V3.
    #[cfg(feature = "nightly")]
    pub fn get_simple_nightshade_layout_testonly() -> ShardLayout {
        ShardLayout::v1(
            vec![
                "aurora",
                "aurora-0",
                "game.hot.tg",
                "kkuuue2akv_1630967379.near",
                "nightly",
                "tge-lockup.sweat",
            ]
            .into_iter()
            .map(|s| s.parse().unwrap())
            .collect(),
            Some(vec![
                vec![0],
                vec![1],
                vec![2],
                vec![3],
                vec![4, 5],
                vec![6],
            ]),
            4,
        )
    }

    /// Given a parent shard id, return the shard uids for the shards in the current shard layout that
    /// are split from this parent shard. If this shard layout has no parent shard layout, return None
    pub fn get_children_shards_uids(&self, parent_shard_id: ShardId) -> Option<Vec<ShardUId>> {
        self.get_children_shards_ids(parent_shard_id).map(|shards| {
            shards
                .into_iter()
                .map(|id| ShardUId::from_shard_id_and_layout(id, self))
                .collect()
        })
    }

    /// Given a parent shard id, return the shard ids for the shards in the current shard layout that
    /// are split from this parent shard. If this shard layout has no parent shard layout, return None
    pub fn get_children_shards_ids(&self, parent_shard_id: ShardId) -> Option<Vec<ShardId>> {
        match self {
            Self::V1(v1) => match &v1.shards_split_map {
                Some(shards_split_map) => shards_split_map.get(parent_shard_id as usize).cloned(),
                None => None,
            },
        }
    }

    /// Return the parent shard id for a given shard in the shard layout
    /// Only calls this function for shard layout that has parent shard layouts
    /// Returns error if `shard_id` is an invalid shard id in the current layout
    /// Panics if `self` has no parent shard layout
    pub fn get_parent_shard_id(&self, shard_id: ShardId) -> Result<ShardId, ShardLayoutError> {
        if !self.shard_ids().any(|id| id == shard_id) {
            return Err(ShardLayoutError::InvalidShardIdError { shard_id });
        }
        let parent_shard_id = match self {
            Self::V1(v1) => match &v1.to_parent_shard_map {
                // we can safely unwrap here because the construction of to_parent_shard_map guarantees
                // that every shard has a parent shard
                Some(to_parent_shard_map) => *to_parent_shard_map.get(shard_id as usize).unwrap(),
                None => panic!("shard_layout has no parent shard"),
            },
        };
        Ok(parent_shard_id)
    }

    #[inline]
    pub fn version(&self) -> ShardVersion {
        match self {
            Self::V1(v1) => v1.version,
        }
    }

    fn num_shards(&self) -> NumShards {
        match self {
            Self::V1(v1) => (v1.boundary_accounts.len() + 1) as NumShards,
        }
    }

    pub fn shard_ids(&self) -> impl Iterator<Item = ShardId> {
        0..self.num_shards()
    }

    /// Returns an iterator that iterates over all the shard uids for all the
    /// shards in the shard layout
    pub fn shard_uids(&self) -> impl Iterator<Item = ShardUId> + '_ {
        self.shard_ids()
            .map(|shard_id| ShardUId::from_shard_id_and_layout(shard_id, self))
    }
}

/// Maps an account to the shard that it belongs to given a shard_layout
/// For V0, maps according to hash of account id
/// For V1, accounts are divided to ranges, each range of account is mapped to a shard.
pub fn account_id_to_shard_id(account_id: &AccountId, shard_layout: &ShardLayout) -> ShardId {
    match shard_layout {
        ShardLayout::V1(ShardLayoutV1 {
            boundary_accounts, ..
        }) => {
            // Note: As we scale up the number of shards we can consider
            // changing this method to do a binary search rather than linear
            // scan. For the time being, with only 4 shards, this is perfectly fine.
            let mut shard_id: ShardId = 0;
            for boundary_account in boundary_accounts {
                if account_id < boundary_account {
                    break;
                }
                shard_id += 1;
            }
            shard_id
        }
    }
}

/// Maps an account to the shard that it belongs to given a shard_layout
pub fn account_id_to_shard_uid(account_id: &AccountId, shard_layout: &ShardLayout) -> ShardUId {
    ShardUId::from_shard_id_and_layout(
        account_id_to_shard_id(account_id, shard_layout),
        shard_layout,
    )
}

/// ShardUId is an unique representation for shards from different shard layout
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Hash,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    ProtocolSchema,
)]
pub struct ShardUId {
    pub version: ShardVersion,
    pub shard_id: u32,
}

impl ShardUId {
    pub fn single_shard() -> Self {
        Self {
            version: 0,
            shard_id: 0,
        }
    }

    /// Byte representation of the shard uid
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut res = [0; 8];
        res[0..4].copy_from_slice(&u32::to_le_bytes(self.version));
        res[4..].copy_from_slice(&u32::to_le_bytes(self.shard_id));
        res
    }

    pub fn next_shard_prefix(shard_uid_bytes: &[u8; 8]) -> [u8; 8] {
        let mut result = *shard_uid_bytes;
        for i in (0..8).rev() {
            if result[i] == u8::MAX {
                result[i] = 0;
            } else {
                result[i] += 1;
                return result;
            }
        }
        panic!("Next shard prefix for shard bytes {shard_uid_bytes:?} does not exist");
    }

    /// Constructs a shard uid from shard id and a shard layout
    pub fn from_shard_id_and_layout(shard_id: ShardId, shard_layout: &ShardLayout) -> Self {
        assert!(shard_layout.shard_ids().any(|i| i == shard_id));
        Self {
            shard_id: shard_id as u32,
            version: shard_layout.version(),
        }
    }

    /// Returns shard id
    pub fn shard_id(&self) -> ShardId {
        ShardId::from(self.shard_id)
    }
}

impl TryFrom<&[u8]> for ShardUId {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    /// Deserialize `bytes` to shard uid
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 8 {
            return Err("incorrect length for ShardUId".into());
        }
        let version = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let shard_id = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        Ok(Self { version, shard_id })
    }
}

/// Returns the byte representation for (block, shard_uid)
pub fn get_block_shard_uid(block_hash: &CryptoHash, shard_uid: &ShardUId) -> Vec<u8> {
    let mut res = Vec::with_capacity(40);
    res.extend_from_slice(block_hash.as_ref());
    res.extend_from_slice(&shard_uid.to_bytes());
    res
}

/// Deserialize from a byte representation to (block, shard_uid)
#[allow(unused)]
pub fn get_block_shard_uid_rev(
    key: &[u8],
) -> Result<(CryptoHash, ShardUId), Box<dyn std::error::Error + Send + Sync>> {
    if key.len() != 40 {
        return Err(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid key length").into(),
        );
    }
    let block_hash = CryptoHash::try_from(&key[..32])?;
    let shard_id = ShardUId::try_from(&key[32..])?;
    Ok((block_hash, shard_id))
}

impl fmt::Display for ShardUId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "s{}.v{}", self.shard_id, self.version)
    }
}

impl fmt::Debug for ShardUId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl str::FromStr for ShardUId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (shard_str, version_str) = s
            .split_once(".")
            .ok_or_else(|| "shard version and number must be separated by \".\"".to_string())?;

        let version = version_str
            .strip_prefix("v")
            .ok_or_else(|| "shard version must start with \"v\"".to_string())?
            .parse::<ShardVersion>()
            .map_err(|e| format!("shard version after \"v\" must be a number, {e}"))?;

        let shard_str = shard_str
            .strip_prefix("s")
            .ok_or_else(|| "shard id must start with \"s\"".to_string())?;
        let shard_id = shard_str
            .parse::<u32>()
            .map_err(|e| format!("shard id after \"s\" must be a number, {e}"))?;

        Ok(ShardUId { shard_id, version })
    }
}

impl<'de> serde::Deserialize<'de> for ShardUId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ShardUIdVisitor)
    }
}

impl serde::Serialize for ShardUId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct ShardUIdVisitor;
impl<'de> serde::de::Visitor<'de> for ShardUIdVisitor {
    type Value = ShardUId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "either string format of `ShardUId` like 's0.v3' for shard 0 version 3, or a map"
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse().map_err(|e| E::custom(e))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        // custom struct deserialization for backwards compatibility
        // TODO(#7894): consider removing this code after checking
        // `ShardUId` is nowhere serialized in the old format
        let mut version = None;
        let mut shard_id = None;

        while let Some((field, value)) = map.next_entry()? {
            match field {
                "version" => version = Some(value),
                "shard_id" => shard_id = Some(value),
                _ => {
                    return Err(serde::de::Error::unknown_field(
                        field,
                        &["version", "shard_id"],
                    ))
                }
            }
        }

        match (version, shard_id) {
            (None, _) => Err(serde::de::Error::missing_field("version")),
            (_, None) => Err(serde::de::Error::missing_field("shard_id")),
            (Some(version), Some(shard_id)) => Ok(ShardUId { version, shard_id }),
        }
    }
}
