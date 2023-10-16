use crate::class_file_error::{ClassFileError, Result};

//https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.1-200-B.2
#[derive(Debug, PartialEq, Default, strum_macros::Display)]
#[allow(dead_code)]
pub enum ClassFileVersion {
    Jdk1_1,
    Jdk1_2,
    Jdk1_3,
    Jdk1_4,
    Jdk5,
    Jdk6,
    Jdk7,
    #[default]
    Jdk8,
    Jdk9,
    Jdk10,
    Jdk11,
    Jdk12,
    Jdk13,
    Jdk14,
    Jdk15,
    Jdk16,
    Jdk17,
    Jdk18,
    Jdk19,
    Jdk20,
    Jdk21,
}
impl ClassFileVersion {
    pub fn version(&self) -> (u16, u16) {
        match self {
            ClassFileVersion::Jdk1_1 => (45,45),
            ClassFileVersion::Jdk1_2 => (45,46),
            ClassFileVersion::Jdk1_3 => (45,47),
            ClassFileVersion::Jdk1_4 => (45,48),
            ClassFileVersion::Jdk5 => (45,49),
            ClassFileVersion::Jdk6 => (45,50),
            ClassFileVersion::Jdk7 => (45,51),
            ClassFileVersion::Jdk8 => (45,52),
            ClassFileVersion::Jdk9 => (45,53),
            ClassFileVersion::Jdk10 => (45,54),
            ClassFileVersion::Jdk11 => (45,55),
            ClassFileVersion::Jdk12 => (45,56),
            ClassFileVersion::Jdk13 => (45,57),
            ClassFileVersion::Jdk14 => (45,58),
            ClassFileVersion::Jdk15 => (45,59),
            ClassFileVersion::Jdk16 => (45,60),
            ClassFileVersion::Jdk17 => (45,61),
            ClassFileVersion::Jdk18 => (45,62),
            ClassFileVersion::Jdk19 => (45,63),
            ClassFileVersion::Jdk20 => (45,64),
            ClassFileVersion::Jdk21 => (45,65),
        }
    }
    /// Creates a version from the major and minor versions specified in the class file
    pub fn new(major: u16, minor: u16) -> Result<ClassFileVersion> {
        match major {
            45 => Ok(ClassFileVersion::Jdk1_1),
            46 => Ok(ClassFileVersion::Jdk1_2),
            47 => Ok(ClassFileVersion::Jdk1_3),
            48 => Ok(ClassFileVersion::Jdk1_4),
            49 => Ok(ClassFileVersion::Jdk5),
            50 => Ok(ClassFileVersion::Jdk6),
            51 => Ok(ClassFileVersion::Jdk7),
            52 => Ok(ClassFileVersion::Jdk8),
            53 => Ok(ClassFileVersion::Jdk9),
            54 => Ok(ClassFileVersion::Jdk10),
            55 => Ok(ClassFileVersion::Jdk11),
            56 => Ok(ClassFileVersion::Jdk12),
            57 => Ok(ClassFileVersion::Jdk13),
            58 => Ok(ClassFileVersion::Jdk14),
            59 => Ok(ClassFileVersion::Jdk15),
            60 => Ok(ClassFileVersion::Jdk16),
            61 => Ok(ClassFileVersion::Jdk17),
            62 => Ok(ClassFileVersion::Jdk18),
            63 => Ok(ClassFileVersion::Jdk19),
            64 => Ok(ClassFileVersion::Jdk20),
            65 => Ok(ClassFileVersion::Jdk21),
            _ => Err(ClassFileError::UnsupportedVersion(major, minor)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{class_file_error::ClassFileError, class_file_version::ClassFileVersion};

    #[test]
    fn can_parse_known_versions() {
        assert_eq!(
            ClassFileVersion::Jdk8,
            ClassFileVersion::new(52, 45).unwrap()
        );
    }

    #[test]
    fn get_version_of_known_jdk() {
        assert_eq!(
            ClassFileVersion::Jdk8.version(),
             (45,52)
        );
    }

    #[test]
    fn can_parse_future_versions() {
        assert_eq!(
            Err(ClassFileError::UnsupportedVersion(99, 65535)),
            ClassFileVersion::new(99, 65535),
        );
    }
}
