use std::collections::HashMap;

use semver::Version;

fn default_aspect_ratio() -> String {
    "16:9".into()
}

/// Defines that this Vid module is a project, aka a video itself. If you 
/// intend to create reusable components for other videos, use the `package` 
/// key instead
#[derive(serde::Deserialize)]
pub struct DefProject {
    /// Project name. Must be a valid identifier, like `my_awesome_video`
    pub name: String,
    /// Aspect ratio for the project. Should be in the format `4:3`. Defaults to 
    /// `16:9`
    #[serde(default = "default_aspect_ratio")]
    pub aspect_ratio: String,
}

/// Defines that this Vid module is a package aka library. In other words, it 
/// is not a full video itself, but instead features reusable components for 
/// making other videos
#[derive(serde::Deserialize)]
pub struct DefPackage {
    /// Package name. Must be a valid identifier, like `std` or `my_video_tools`
    pub name: String,
    /// Package description
    pub description: String,
    /// Package version. This should follow the [RomVer](https://romversioning.github.io/romver/) 
    /// standard
    pub version: Version,
    /// URL to a GitHub repository or equivalent place where the source code 
    /// and/or docs and/or homepage for this package can be found
    pub repository: Option<String>,
}

#[derive(serde::Deserialize)]
#[allow(non_camel_case_types)]
pub enum VidType {
    project(DefProject),
    package(DefPackage),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VidToml {
    #[serde(flatten)]
    pub ty: VidType,
    #[serde(default)]
    pub packages: HashMap<String, String>,
}

impl VidToml {
    #[cfg(test)]
    pub fn new_test(name: &str) -> Self {
        Self {
            ty: VidType::project(DefProject {
                name: name.to_string(),
                aspect_ratio: default_aspect_ratio()
            }),
            // todo: add std package here
            packages: Default::default(),
        }
    }
}
