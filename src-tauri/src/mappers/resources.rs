use crate::domain::resource::{Resource, ResourceStatus};
use console_api::resources;
use console_api::resources::resource::kind;

pub fn map_to_domain_resource(resource: &resources::Resource) -> Option<Resource> {
    let mut location = resource.location.as_ref().map(|loc| {
        let full_path = loc.file();
        let path = std::path::Path::new(full_path);

        let directory = path.parent().and_then(|p| p.to_str()).unwrap_or("");
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

        format!(
            "{}{}<b>{}</b>:<b>{}</b>:{}",
            directory,
            std::path::MAIN_SEPARATOR,
            filename,
            loc.line(),
            loc.column()
        )
    });
    if location.is_none() {
        location = Some("Unknown".into());
    }

    let resource_type = {
        if let Some(kind) = resource.kind.as_ref() {
            if let Some(kind_of_kind) = kind.kind.as_ref() {
                match kind_of_kind {
                    kind::Kind::Known(i) => Some(
                        kind::Known::try_from(*i)
                            .ok()
                            .unwrap()
                            .as_str_name()
                            .to_string(),
                    ),
                    kind::Kind::Other(s) => Some(s.clone()),
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    let id = resource.id.as_ref().map(|v| v.id)?;
    let target = Some(resource.concrete_type.clone());

    Some(Resource {
        app_name: None,
        resource_type,
        id,
        status: ResourceStatus::Ready,
        target,
        duration: None,
        location,
        attributes: None,
    })
}
