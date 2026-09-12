use board_protocol::models::room::upload_file_policy::{
    UploadFilePolicyError, UploadFileTypeMode, UploadFileTypePolicy, extension_of,
    normalize_upload_file_extensions, normalize_upload_file_type, upload_file_type_violation,
};

#[test]
fn extension_of_extracts_lowercased_last_segment() {
    assert_eq!(extension_of("photo.PNG").as_deref(), Some("png"));
    assert_eq!(extension_of("a/b/c.EXE").as_deref(), Some("exe"));
    assert_eq!(extension_of("a\\b\\c.EXE").as_deref(), Some("exe"));
    assert_eq!(extension_of("file.tar.gz").as_deref(), Some("gz"));
}

#[test]
fn extension_of_handles_missing_or_empty_extensions() {
    assert_eq!(extension_of("noext"), None);
    assert_eq!(extension_of("archive.tar."), None);
    assert_eq!(extension_of(""), None);
}

fn policy(mode: UploadFileTypeMode, extensions: &[&str]) -> UploadFileTypePolicy {
    UploadFileTypePolicy {
        mode,
        extensions: extensions.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn any_mode_permits_everything() {
    let p = policy(UploadFileTypeMode::Any, &[]);
    assert!(p.permits("anything.exe"));
    assert!(p.permits("noext"));
}

#[test]
fn allow_mode_only_permits_listed_extensions() {
    let p = policy(UploadFileTypeMode::Allow, &["png", "pdf"]);
    assert!(p.permits("photo.png"));
    assert!(p.permits("PHOTO.PNG"));
    assert!(p.permits("doc.pdf"));
    assert!(!p.permits("evil.exe"));
    assert!(!p.permits("noext"));
}

#[test]
fn deny_mode_rejects_only_listed_extensions() {
    let p = policy(UploadFileTypeMode::Deny, &["exe", "sh"]);
    assert!(!p.permits("run.exe"));
    assert!(!p.permits("run.sh"));
    assert!(p.permits("notes.txt"));
    assert!(p.permits("noext"));
}

#[test]
fn normalization_lowercases_strips_dots_and_dedupes() {
    let normalized = normalize_upload_file_extensions(&[
        ".PNG".to_string(),
        "Pdf".to_string(),
        "png".to_string(),
    ])
    .expect("valid list");
    assert_eq!(normalized, vec!["png".to_string(), "pdf".to_string()]);
}

#[test]
fn normalization_rejects_invalid_entries_and_overflow() {
    assert_eq!(
        normalize_upload_file_extensions(&["a$b".to_string()]),
        Err(UploadFilePolicyError::InvalidExtension("a$b".to_string()))
    );
    assert_eq!(
        normalize_upload_file_extensions(&[".".to_string()]),
        Err(UploadFilePolicyError::InvalidExtension(".".to_string()))
    );
    assert_eq!(
        normalize_upload_file_extensions(&["toolongextensionx".to_string()]),
        Err(UploadFilePolicyError::InvalidExtension(
            "toolongextensionx".to_string()
        ))
    );
    let too_many: Vec<String> = (0..65).map(|i| format!("e{i}")).collect();
    assert_eq!(
        normalize_upload_file_extensions(&too_many),
        Err(UploadFilePolicyError::TooManyExtensions)
    );
}

#[test]
fn policy_type_normalization_clears_list_for_any_and_requires_entries_otherwise() {
    let any =
        normalize_upload_file_type(policy(UploadFileTypeMode::Any, &["exe"])).expect("any valid");
    assert_eq!(any.extensions, Vec::<String>::new());

    assert_eq!(
        normalize_upload_file_type(policy(UploadFileTypeMode::Allow, &[])),
        Err(UploadFilePolicyError::EmptyExtensions)
    );
    assert_eq!(
        normalize_upload_file_type(policy(UploadFileTypeMode::Deny, &[])),
        Err(UploadFilePolicyError::EmptyExtensions)
    );

    let allow =
        normalize_upload_file_type(policy(UploadFileTypeMode::Allow, &[".PdF", "pdf"])).unwrap();
    assert_eq!(allow.extensions, vec!["pdf".to_string()]);
}

#[test]
fn violation_message_carries_the_file_name() {
    assert_eq!(
        upload_file_type_violation("evil.exe"),
        "File type not allowed by room policy: evil.exe"
    );
}
