use rhythm_core::{
    project::{Project, ProjectSettings},
    serialization::{
        ProjectFileV1, migrate_project_file_to_current, parse_project_file_v1,
        validate_project_file_v1_candidate,
    },
    time::{BpmMicros, GridOffsetNs, TempoMap, TimeSignature},
};

#[test]
fn committed_minimal_v1_fixture_loads_as_a_valid_semantic_project() {
    let bytes = include_bytes!("fixtures/minimal_v1.rhfx");

    let candidate = parse_project_file_v1(bytes).expect("fixture is UTF-8 schema V1 JSON");
    let migrated = migrate_project_file_to_current(candidate).expect("fixture is schema V1");
    let validated =
        validate_project_file_v1_candidate(migrated).expect("fixture is semantically valid");

    let expected = ProjectFileV1::new(
        Project::new(
            "Minimal V1",
            ProjectSettings::default(),
            TempoMap::with_initial_tempo(
                GridOffsetNs::new(0),
                BpmMicros::new(120_000_000).expect("valid 120 BPM"),
                TimeSignature::new(4, 4).expect("valid meter"),
            ),
        ),
        "0.1.0",
    );

    assert_eq!(validated, expected);
}
