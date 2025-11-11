use std::fmt::Display;

///This holds the epoch, version, and pkgver
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Version {
    pub raw: String,
    pub epoch: u32,
    major: String,
    minor: String,
    patch: String,
    revision: String, //this is for anything after the 3rd dot, may have multiple other dots, each change is considered revision change
    pub pkgver: u32,
}

impl From<&str> for Version {
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

        let parts: Vec<&str> = version_str.splitn(4, '.').collect();
        let mut major = "".to_string();
        let mut minor = "".to_string();
        let mut patch = "".to_string();
        let mut revision = "".to_string();

        for p in parts.iter().enumerate() {
            match p.0 {
                0 => major = p.1.to_string(),
                1 => minor = p.1.to_string(),
                2 => patch = p.1.to_string(),
                3 => revision = p.1.to_string(),
                _ => (),
            }
        }

        Version {
            raw,
            epoch,
            major,
            minor,
            patch,
            revision,
            pkgver,
        }
    }
}

impl Version {
    pub fn change_type(&self, other: &Version) -> ChangeType {
        if self.epoch != other.epoch {
            ChangeType::Epoch
        } else if self.major != other.major {
            ChangeType::Major
        } else if self.minor != other.minor {
            ChangeType::Minor
        } else if self.patch != other.patch {
            ChangeType::Patch
        } else if self.revision != other.revision {
            ChangeType::Revision
        } else {
            ChangeType::Pkgver
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash, Ord)]
pub enum ChangeType {
    Pkgver,   //build change only
    Revision, //smaller than patch change, anything after 3rd dot
    Patch,    //third dot change
    Minor,    //second dot change
    Major,    //first dot change
    Epoch,    //change to force update, even if it may look smaller
}
impl Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
