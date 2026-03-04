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
    name: String,
    /// Aspect ratio for the project. Should be in the format `4:3`. Defaults to 
    /// `16:9`
    #[serde(default = "default_aspect_ratio")]
    aspect_ratio: String,
}

/// Defines that this Vid module is a package aka library. In other words, it 
/// is not a full video itself, but instead features reusable components for 
/// making other videos
#[derive(serde::Deserialize)]
pub struct DefPackage {
    /// Package name. Must be a valid identifier, like `std` or `my_video_tools`
    name: String,
    /// Package description
    description: String,
    /// Package version. This should follow the [RomVer](https://romversioning.github.io/romver/) 
    /// standard
    version: Version,
    /// URL to a GitHub repository or equivalent place where the source code 
    /// and/or docs and/or homepage for this package can be found
    repository: Option<String>,
}

#[derive(serde::Deserialize)]
pub enum VidType {
    project(DefProject),
    package(DefPackage),
}

#[derive(serde::Deserialize)]
pub struct UsePackage {
    
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VidToml {
    #[serde(flatten)]
    ty: VidType,
    packages: 
}
