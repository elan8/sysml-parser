use sysml_v2_parser::ast::{
    ActionDefBody, OccurrenceBodyElement, OccurrenceUsageBody, PackageBody, PackageBodyElement,
    PartDefBody, PartDefBodyElement, PartUsageBody, PartUsageBodyElement, RequirementDefBody,
    RequirementDefBodyElement, RootElement, StateDefBody, StateDefBodyElement,
};
use sysml_v2_parser::{parse, parse_with_diagnostics};

fn package_elements(input: &str) -> Vec<sysml_v2_parser::Node<PackageBodyElement>> {
    let root = parse(input).expect("input should parse");
    let pkg = match &root.elements[0].value {
        RootElement::Package(p) => &p.value,
        other => panic!("expected package, got {other:?}"),
    };
    match &pkg.body {
        PackageBody::Brace { elements } => elements.clone(),
        _ => panic!("expected brace package body"),
    }
}

#[test]
fn individual_part_definition_and_usage_parse_as_parts() {
    let input = "package P {\nindividual part def 'Neil Armstrong' :> Astronaut { }\nindividual part crewMember : Astronaut { }\n}";
    let elements = package_elements(input);

    match &elements[0].value {
        PackageBodyElement::PartDef(def) => {
            assert!(def.value.is_individual);
            assert_eq!(def.value.identification.name.as_deref(), Some("Neil Armstrong"));
        }
        other => panic!("expected individual part def, got {other:?}"),
    }

    match &elements[1].value {
        PackageBodyElement::PartUsage(usage) => {
            assert!(usage.value.is_individual);
            assert_eq!(usage.value.name, "crewMember");
        }
        other => panic!("expected individual part usage, got {other:?}"),
    }
}

#[test]
fn requirement_usage_supports_trailing_subsets_after_body() {
    let input = "package P {\npart def Mission {\nrequirement goals[1..*] : Goal;\n}\npart def ApolloMission :> Mission {\nrequirement goToMoon : Goal {\ndoc /* Perform a crewed lunar landing and return to Earth. */\n} :> goals;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let part = elements
        .iter()
        .find_map(|e| match &e.value {
            PackageBodyElement::PartDef(def)
                if def.value.identification.name.as_deref() == Some("ApolloMission") =>
            {
                Some(&def.value)
            }
            _ => None,
        })
        .expect("ApolloMission part def should be present");
    let PartDefBody::Brace { elements } = &part.body else {
        panic!("expected part body");
    };
    let req = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::RequirementUsage(req) => Some(&req.value),
            _ => None,
        });
    let req = req.expect("requirement usage should parse in part body");
    assert_eq!(req.subsets.as_deref(), Some("goals"));
}

#[test]
fn exhibit_state_body_supports_unnamed_and_accepting_transitions() {
    let input = "package P {\npart def Mission {\nexhibit state phases {\nstate initial : Initial;\nstate launch : Launch;\ntransition first initial then launch;\ntransition first launch accept LaunchDone then initial {\ndoc /* Example transition body. */\n}\n}\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let mission = match &elements[0].value {
        PackageBodyElement::PartDef(def) => &def.value,
        _ => panic!("expected part def"),
    };
    let PartDefBody::Brace { elements } = &mission.body else {
        panic!("expected part body");
    };
    let exhibit = elements
        .iter()
        .find_map(|e| match &e.value {
            PartDefBodyElement::ExhibitState(exhibit) => Some(&exhibit.value),
            _ => None,
        })
        .expect("exhibit state should be present");
    let StateDefBody::Brace { elements } = &exhibit.body else {
        panic!("expected exhibit state body");
    };
    let transitions: Vec<_> = elements
        .iter()
        .filter_map(|e| match &e.value {
            StateDefBodyElement::Transition(t) => Some(&t.value),
            _ => None,
        })
        .collect();
    assert_eq!(transitions.len(), 2);
    assert_eq!(transitions[0].name, None);
    assert_eq!(transitions[1].name, None);
}

#[test]
fn timeslice_and_snapshot_parse_inside_part_and_occurrence_bodies() {
    let input = "package P {\nindividual part def MissionIndividual :> Mission;\nindividual part mission : MissionIndividual {\ntimeslice liftoff {\nsnapshot atT0 {\nattribute missionTime = 0;\n}\n}\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "unexpected diagnostics: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    let usage = match &elements[1].value {
        PackageBodyElement::PartUsage(usage) => &usage.value,
        _ => panic!("expected part usage"),
    };
    let PartUsageBody::Brace { elements } = &usage.body else {
        panic!("expected part usage body");
    };
    let timeslice = elements
        .iter()
        .find_map(|e| match &e.value {
            PartUsageBodyElement::OccurrenceUsage(occ) => Some(&occ.value),
            _ => None,
        })
        .expect("timeslice should parse in part body");
    assert_eq!(timeslice.portion_kind.as_deref(), Some("timeslice"));
    let OccurrenceUsageBody::Brace { elements } = &timeslice.body else {
        panic!("expected timeslice body");
    };
    let snapshot = elements
        .iter()
        .find_map(|e| match &e.value {
            OccurrenceBodyElement::OccurrenceUsage(occ) => Some(&occ.value),
            _ => None,
        })
        .expect("snapshot should parse in timeslice body");
    assert_eq!(snapshot.portion_kind.as_deref(), Some("snapshot"));
}

#[test]
fn rationale_and_refinement_annotations_stay_localized() {
    let input = "package P {\naction def PerformCrewIngress {\nout isCrewAboard: Boolean;\n@Rationale { }\n#refinement dependency PerformCrewIngress to OperationsPackage::TransferCrewToVehicle;\n}\nrequirement def R {\n@Rationale { }\n#refinement dependency 'HLR-R001' to CapabilitiesPackage::DeepSpaceHabitationAndLifeSupport;\n}\n}";
    let result = parse_with_diagnostics(input);
    assert!(
        result.errors.is_empty(),
        "annotations should parse without recovery cascades: {:?}",
        result.errors
    );

    let pkg = match &result.root.elements[0].value {
        RootElement::Package(p) => &p.value,
        _ => panic!("expected package"),
    };
    let PackageBody::Brace { elements } = &pkg.body else {
        panic!("expected brace body");
    };
    match &elements[0].value {
        PackageBodyElement::ActionDef(action) => {
            let ActionDefBody::Brace { elements } = &action.value.body else {
                panic!("expected action body");
            };
            assert!(elements.iter().any(|e| matches!(e.value, sysml_v2_parser::ActionDefBodyElement::Annotation(_))));
        }
        _ => panic!("expected action def"),
    }
    match &elements[1].value {
        PackageBodyElement::RequirementDef(req) => {
            let RequirementDefBody::Brace { elements } = &req.value.body else {
                panic!("expected requirement body");
            };
            assert!(elements.iter().any(|e| matches!(e.value, RequirementDefBodyElement::Annotation(_))));
        }
        _ => panic!("expected requirement def"),
    }
}

#[test]
fn quoted_requirement_identifier_parses() {
    let input = "package P {\nrequirement def <'HLR-R001'> CrewReturnSafetyRequirement { }\n}";
    let elements = package_elements(input);
    match &elements[0].value {
        PackageBodyElement::RequirementDef(req) => {
            assert_eq!(
                req.value.identification.short_name.as_deref(),
                Some("HLR-R001")
            );
            assert_eq!(
                req.value.identification.name.as_deref(),
                Some("CrewReturnSafetyRequirement")
            );
        }
        other => panic!("expected requirement def, got {other:?}"),
    }
}
