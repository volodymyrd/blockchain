// TODO: consider wrapping these types to prevent interactions
// with other time libraries, especially to prevent the direct access
// to the realtime (i.e. not through the Clock).
pub type Instant = std::time::Instant;
// TODO: OffsetDateTime stores the timestamp in a decomposed form of
// (year,month,day,hour,...). If we find it inefficient, we should
// probably migrate to a pure UNIX timestamp and convert is to datetime
// only when needed.
pub type Utc = time::OffsetDateTime;
pub type Duration = time::Duration;
