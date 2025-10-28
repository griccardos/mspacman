use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PacmanVersion {
    pub raw: String,
    pub epoch: u32,
    pub version: Version,
    pub pkgver: u32,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash, Ord)]
pub enum ChangeType {
    Pkgver,
    Revision,
    Patch,
    Minor,
    Major,
    Epoch,
}
impl Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn change_type(current: &str, new: &str) -> ChangeType {
    let curr = PacmanVersion::from(current);
    let new = PacmanVersion::from(new);

    if curr.epoch != new.epoch {
        ChangeType::Epoch
    } else if curr.version != new.version {
        match (&curr.version, &new.version) {
            (
                Version::Semver4(c_maj, c_min, c_pat, _),
                Version::Semver4(n_maj, n_min, n_pat, _),
            ) => {
                if n_maj > c_maj {
                    ChangeType::Major
                } else if n_min > c_min {
                    ChangeType::Minor
                } else if n_pat > c_pat {
                    ChangeType::Patch
                } else {
                    ChangeType::Revision
                }
            }
            (Version::Semver3(c_maj, c_min, _), Version::Semver3(n_maj, n_min, _)) => {
                if n_maj > c_maj {
                    ChangeType::Major
                } else if n_min > c_min {
                    ChangeType::Minor
                } else {
                    ChangeType::Patch
                }
            }
            (Version::Semver2(c_maj, _), Version::Semver2(n_maj, _)) => {
                if n_maj > c_maj {
                    ChangeType::Major
                } else {
                    ChangeType::Minor
                }
            }
            _ => ChangeType::Major,
        }
    } else {
        ChangeType::Pkgver
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Version {
    Semver4(u32, u32, u32, String),
    Semver3(u32, u32, u32),
    Semver2(u32, u32),
    Other(String),
}

impl From<&str> for PacmanVersion {
    fn from(s: &str) -> Self {
        let raw = s.to_string();
        let mut parts = s.split(':');
        let (epoch, rest) = if let Some(epoch_str) = parts.next() {
            if let Ok(epoch) = epoch_str.parse::<u32>() {
                (epoch, parts.next().unwrap_or(""))
            } else {
                (0, s)
            }
        } else {
            (0, s)
        };

        let (version_str, pkgver) = if let Some((ver, pkgver)) = rest.rsplit_once('-') {
            (ver, pkgver.parse().unwrap_or(0))
        } else {
            (rest, 0)
        };

        let version = version_str.into();

        PacmanVersion {
            raw,
            epoch,
            version,
            pkgver,
        }
    }
}

impl From<&str> for Version {
    fn from(s: &str) -> Self {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() == 4 {
            if let (Ok(major), Ok(minor), Ok(patch), build) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
                parts[3],
            ) {
                return Version::Semver4(major, minor, patch, build.to_string());
            }
        } else if parts.len() == 3 {
            if let (Ok(major), Ok(minor), Ok(patch)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                return Version::Semver3(major, minor, patch);
            }
        } else if parts.len() == 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                return Version::Semver2(major, minor);
            }
        }
        Version::Other(s.to_string())
    }
}
